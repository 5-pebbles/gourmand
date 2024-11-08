use core::{
    arch::asm,
    cmp::max,
    ffi::c_void,
    ptr::{null, null_mut},
    slice,
};

use crate::{
    elf::{
        dynamic_array::{
            DynamicArrayIter, DT_RELA, DT_RELAENT, DT_RELASZ, DT_STRTAB, DT_SYMENT, DT_SYMTAB,
        },
        program_header::ProgramHeader,
        relocate::Rela,
        symbol::Symbol,
        thread_local_storage::ThreadControlBlock,
    },
    io_macros::*,
    shared_object::SharedObject,
    syscall_assert, syscall_debug_assert,
};

#[naked]
#[no_mangle]
pub(super) unsafe extern "C" fn _start() -> ! {
    asm!("mov rdi, rsp",
        "and rsp, -16", // !0b1111
        "call {}",
        "mov rdx, 0",
        "jmp rax",
        sym crate::rust_main,
        options(noreturn)
    );
}

// This function uses a lot of inline asm and architecture specific code, which is why it's in arch...
pub(crate) unsafe fn relocate(shared_object: &SharedObject) {
    let base_address = shared_object.base.addr();

    // Variables in relocation formulae:
    // - A(rela.r_addend): This is the addend used to compute the value of the relocatable field.
    // - B(base_address): This is the base address at which a shared object has been loaded into memory during execution.
    // - G(??): This is the offset into the global offset table at which the address of the relocation entry’s symbol will reside during execution.
    // - GOT(global_offset_table_address): This is the address of the global offset table.
    // - L(??): ??
    // - P(relocate_address): This is the address of the storage unit being relocated.
    // - S(symbol.st_value): This is the value of the symbol table entry indexed at `rela.r_sym()`.
    //   NOTE: In the ELF specification `S` is equal to (symbol.st_value + base_address) but that doesn't make any sense to me.
    // - Z(??): ??

    // x86_64 relocation types:
    /// | None
    const R_X86_64_NONE: u32 = 0;
    /// S + B + A | u64
    const R_X86_64_64: u32 = 1;
    /// S + B + A - P | u32
    const R_X86_64_PC32: u32 = 2;
    /// G + A | u32
    const R_X86_64_GOT32: u32 = 3;
    /// L + A - P | u32
    const R_X86_64_PLT32: u32 = 4;
    /// | None
    const R_X86_64_COPY: u32 = 5;
    /// S + B | u64
    const R_X86_64_GLOB_DAT: u32 = 6;
    /// S + B | u64
    const R_X86_64_JUMP_SLOT: u32 = 7;
    /// B + A | u64
    const R_X86_64_RELATIVE: u32 = 8;
    /// G + GOT + A - P | u32
    const R_X86_64_GOTPCREL: u32 = 9;
    /// S + B + A | u32
    const R_X86_64_32: u32 = 10;
    /// S + B + A | u32
    const R_X86_64_32S: u32 = 11;
    /// S + B + A | u16
    const R_X86_64_16: u32 = 12;
    /// S + B + A - P | u16
    const R_X86_64_PC16: u32 = 13;
    /// S + B + A | u8
    const R_X86_64_8: u32 = 14;
    /// S + B + A - P | u8
    const R_X86_64_PC8: u32 = 15;
    /// S + B + A - P | u64
    const R_X86_64_PC64: u32 = 24;
    /// S + B + A - GOT | u64
    const R_X86_64_GOTOFF64: u32 = 25;
    /// GOT + A - P | u32
    const R_X86_64_GOTPC32: u32 = 26;
    /// Z + A | u32
    const R_X86_64_SIZE32: u32 = 32;
    /// Z + A | u64
    const R_X86_64_SIZE64: u32 = 33;
    /// The returned value from the function located at (B + A) | u64
    const R_X86_64_IRELATIVE: u32 = 37; // This one is fucking awesome... I mean, it's a little annoying but really cool.

    // You may notice some are missing values; those are part of the Thread-Local Storage ABI see "ELF Handling for Thread-Local Storage":
    const R_X86_64_DTPMOD64: u32 = 16;

    for rela in shared_object.relocations.rela {
        let relocate_address = rela.r_offset.wrapping_add(base_address);
        let symbol = shared_object.symbol_table.get(rela.r_sym() as usize);

        // x86_64 assembly pointer widths:
        // byte  | 8 bits  (1 byte)
        // word  | 16 bits (2 bytes)
        // dword | 32 bits (4 bytes) | "double word"
        // qword | 64 bits (8 bytes) | "quad word"
        match rela.r_type() {
            R_X86_64_64 => {
                let relocate_value = symbol
                    .st_value
                    .wrapping_add(base_address)
                    .wrapping_add_signed(rela.r_addend);
                asm!(
                    "mov qword ptr [{}], {}",
                    in(reg) relocate_address,
                    in(reg) relocate_value,
                    options(nostack, preserves_flags),
                );
            }
            R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                let relocate_value = symbol.st_value.wrapping_add(base_address);
                asm!(
                    "mov qword ptr [{}], {}",
                    in(reg) relocate_address,
                    in(reg) relocate_value,
                    options(nostack, preserves_flags),
                )
            }
            R_X86_64_RELATIVE => {
                let relocate_value = base_address.wrapping_add_signed(rela.r_addend);
                asm!(
                    "mov qword ptr [{}], {}",
                    in(reg) relocate_address,
                    in(reg) relocate_value,
                    options(nostack, preserves_flags),
                );
            }
            R_X86_64_IRELATIVE => {
                let function_pointer = base_address.wrapping_add_signed(rela.r_addend) as *const ();
                let function: extern "C" fn() -> usize = core::mem::transmute(function_pointer);
                let relocate_value = function();
                asm!(
                    "mov qword ptr [{}], {}",
                    in(reg) relocate_address,
                    in(reg) relocate_value,
                    options(nostack, preserves_flags),
                );
            }
            // _ => (),
            _ => {
                syscall_assert!(false, "unsupported relocation");
            }
        }
    }
}

pub(crate) unsafe fn init_thread_local_storage(
    shared_object: &SharedObject,
    pseudorandom_bytes: *const [u8; 16],
) {
    // Thread Local Storage [before Thread Pointer]:
    //                           |----------------------------|         |---------------------|
    //                           |       TCB Alignment        |         |    ...tls-offset    |
    //                       |-> |----------------------------| <----|  |---------------------|
    // |----------|          |   |  ...Static TLS Blocks      |      |- |    tls-offset[-2]   |
    // |   GENt   | <-|   |--|-> |----------------------------| <-|     |---------------------|
    // |----------|   |   |  |   |      Static TLS Block      |   |---- |   tls-offset[-1]    |
    // |  DTV[1]  | --|---|  |   |----------------------------|         |---------------------|
    // |----------|   |      |   |    Static TLS Alignment    |   |---- | Thread Pointer (TP) |
    // |  DTV[2]  |---|------|   |----------------------------| <-|     |---------------------|
    // |----------|   |--------- | Thread Control Block (TCB) |
    // |  DTV[3]  | ----|        |----------------------------|
    // |----------|     |
    // |  DTV[4]  | -|  |   |------------------------------|
    // |----------|  |  |-> | Dynamically−loaded TLS Block |
    // |  DTV...  |  |      |------------------------------|
    // |----------|  |
    //               |      |------------------------------|
    //               |----> | Dynamically−loaded TLS Block |
    //                      |------------------------------|
    let tcb_align = max(
        shared_object.tls.program_header.p_align,
        align_of::<ThreadControlBlock>(),
    );
    let tls_blocks_size_and_align = {
        let address = shared_object.tls.program_header.p_memsz;
        let boundary = shared_object.tls.program_header.p_align;
        (address + (boundary - 1)) & boundary.wrapping_neg()
    };
    let tcb_size = size_of::<ThreadControlBlock>();

    // We use system calls because the allocator itself may use TLS:
    let required_size = tcb_align + tls_blocks_size_and_align + tcb_size;
    let tls_allocation_pointer = mmap(
        required_size,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
    );
    syscall_debug_assert!(tls_allocation_pointer.addr() % tcb_align == 0);

    // Initialize the TLS data from template image:
    slice::from_raw_parts_mut(
        tls_allocation_pointer.byte_add(tcb_align),
        shared_object.tls.program_header.p_filesz,
    )
    .copy_from_slice(slice::from_raw_parts(
        shared_object
            .base
            .byte_add(shared_object.tls.program_header.p_offset) as *const u8,
        shared_object.tls.program_header.p_filesz,
    ));

    // Zero out TLS data beyond `p_filesz`:
    slice::from_raw_parts_mut(
        tls_allocation_pointer
            .byte_add(tcb_align)
            .byte_add(shared_object.tls.program_header.p_filesz),
        shared_object.tls.program_header.p_memsz - shared_object.tls.program_header.p_filesz,
    )
    .fill(0);

    // Initialize the Thread Control Block (TCB):
    let thread_control_block_pointer = tls_allocation_pointer
        .byte_add(tcb_align)
        .byte_add(tls_blocks_size_and_align)
        as *mut ThreadControlBlock;

    let thread_pointer_register: *mut c_void = (*thread_control_block_pointer)
        .thread_pointee
        .as_mut_ptr()
        .cast();

    *thread_control_block_pointer = ThreadControlBlock {
        thread_pointee: [],
        thread_pointer_register,
        dynamic_thread_vector: null_mut(),
        _padding: [0; 3],
        canary: usize::from_ne_bytes(
            (*pseudorandom_bytes)[..size_of::<usize>()]
                .try_into()
                .unwrap(),
        ),
    };

    // Make the thread pointer (which is fs on x86_64) point to the TCB:
    set_thread_pointer(thread_pointer_register);
}

#[inline(always)]
pub(crate) fn write(fd: i32, s: &str) {
    const WRITE: usize = 1;

    let result: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") WRITE => result,
            in("rdi") fd,
            in("rsi") s.as_ptr(),
            in("rdx") s.len(),
            out("rcx") _,
            out("r11") _,
            options(nostack)
        )
    };
    syscall_debug_assert!(result >= 0);
}

// Protection flags:
pub(crate) const PROT_READ: usize = 0x1;
pub(crate) const PROT_WRITE: usize = 0x2;
pub(crate) const PROT_EXEC: usize = 0x4;

// MAP flags:
pub(crate) const MAP_PRIVATE: usize = 0x2;
pub(crate) const MAP_ANONYMOUS: usize = 0x20;

#[inline(always)]
pub(crate) unsafe fn mmap(size: usize, protection_flags: usize, map_flags: usize) -> *mut u8 {
    const MMAP: usize = 9; // I am like 80% sure this is the right system call... :)

    let mut result: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") MMAP => result,
            in("rdi") null_mut::<c_void>(),
            in("rsi") size,
            in("rdx") protection_flags,
            in("r10") map_flags,
            in("r8") -1isize, // file descriptor (-1 for anonymous mapping)
            in("r9") 0usize, // offset
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    syscall_debug_assert!(result >= 0);
    result as *mut u8
}

#[inline(always)]
pub(crate) unsafe fn munmap(pointer: *mut u8, size: usize) {
    const MUNMAP: usize = 11;

    let mut result: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") MUNMAP => result,
            in("rdi") pointer,
            in("rsi") size,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        )
    };
    syscall_debug_assert!(result >= 0);
}

#[inline(always)]
pub(crate) unsafe fn set_thread_pointer(new_pointer: *mut c_void) {
    const ARCH_PRCTL: usize = 158;
    const ARCH_SET_FS: usize = 4098;

    asm!(
        "syscall",
        in("rax") ARCH_PRCTL,
        in("rdi") ARCH_SET_FS,
        in("rsi") new_pointer,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    syscall_debug_assert!(*new_pointer.cast::<*mut c_void>() == new_pointer);
    syscall_debug_assert!(get_thread_pointer() == new_pointer);
}

#[inline(always)]
pub(crate) unsafe fn get_thread_pointer() -> *mut c_void {
    let pointer;
    asm!(
        "mov {}, fs:0",
        out(reg) pointer,
        options(nostack, preserves_flags, readonly)
    );
    pointer
}

#[inline(always)]
pub(crate) fn exit(code: usize) -> ! {
    const EXIT: usize = 60;

    unsafe {
        asm!(
            "syscall",
            in("rax") EXIT,
            in("rdi") code,
            options(noreturn)
        )
    }
}
