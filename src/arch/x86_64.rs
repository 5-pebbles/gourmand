use core::{arch::asm, ffi::c_void, ptr::null_mut};

use crate::linux::{io_macros::*, shared_object::SharedObject};

#[naked]
#[no_mangle]
pub(super) unsafe extern "C" fn _start() -> ! {
    asm!("mov rdi, rsp",
        "and rsp, -16", // !0b1111
        "call {}",
        "mov rdx, 0",
        "jmp rax",
        sym crate::linux::rust_start::rust_start,
        options(noreturn)
    );
}

// This function uses a lot of inline asm and architecture specific code, which is why it's in arch...
pub(crate) unsafe fn relocate(shared_object: &SharedObject) {
    // x86_64 only uses RELAs
    let base_address = shared_object.load_bias.addr();

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
    // TODO: I have no idea how this one works:
    /// INDIRECT (B + A) | u64
    const R_X86_64_IRELATIVE: u32 = 37;
    // You may notice some are missing values; those are part of the Thread-Local Storage ABI and can be found in:
    // 1. "ELF Handling for Thread-Local Storage".
    // 2. "Thread-Local Storage Descriptors for IA32 and AMD64/EM64T".

    for rela in shared_object.rela {
        let relocate_address = rela.r_offset.wrapping_add(base_address);
        let symbol = &*shared_object
            .symbol_table_pointer
            .add(rela.r_sym() as usize);

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
            // _ => (),
            _ => {
                syscall_assert!(false, "unsupported relocation");
            }
        }
    }
}

// https://en.wikipedia.org/wiki/Exit_(system_call)
#[inline(always)]
pub(super) fn exit(code: usize) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") 60,
            in("rdi") code,
            options(noreturn)
        )
    }
}

// https://en.wikipedia.org/wiki/Write_(system_call)
#[inline(always)]
pub(super) fn write(fd: i32, s: &str) -> isize {
    let result: isize;
    unsafe {
        asm!(
            "syscall",
            inout("rax") 1_isize => result,
            in("rdi") fd,
            in("rsi") s.as_ptr(),
            in("rdx") s.len(),
        )
    };
    syscall_debug_assert!(result >= 0);
    result
}

#[inline(always)]
pub unsafe fn mmap(size: usize) -> *mut u8 {
    // Protection flags
    const PROT_READ: usize = 0x1;
    const PROT_WRITE: usize = 0x2;
    const PROT_EXEC: usize = 0x4;

    // MAP flags
    const MAP_PRIVATE: usize = 0x2;
    const MAP_ANONYMOUS: usize = 0x20;

    let mut result: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") 9isize => result, // I am like 80% sure this is the right system call... :)
            in("rdi") null_mut::<c_void>(),
            in("rsi") size,
            in("rdx") PROT_READ | PROT_WRITE,
            in("r10") MAP_PRIVATE | MAP_ANONYMOUS,
            in("r8") -1isize, // file descriptor (-1 for anonymous mapping)
            in("r9") 0usize, // offset
        );
    }
    syscall_debug_assert!(result >= 0);
    result as *mut u8
}

#[inline(always)]
pub unsafe fn munmap(pointer: *mut u8, size: usize) -> isize {
    let mut result: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") 11isize => result,
            in("rdi") pointer,
            in("rsi") size,
        )
    };
    syscall_debug_assert!(result >= 0);
    result
}
