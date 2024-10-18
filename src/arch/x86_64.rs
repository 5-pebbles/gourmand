use core::{
    arch::asm,
    ffi::c_void,
    ptr::{null, null_mut},
};

use crate::{
    elf::{dynamic_array::*, relocate::Rela, symbol::Symbol},
    linux::io_macros::*,
};

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
pub(crate) unsafe fn relocate(base: *mut c_void, dynamic_array_iter: DynamicArrayIter) {
    syscall_debug_assert!(!dynamic_array_iter.clone().any(|i| i.d_tag == DT_REL));
    syscall_debug_assert!(!dynamic_array_iter.clone().any(|i| i.d_tag == DT_RELR));

    // x86_64 only uses RELAs
    let mut rela_pointer: *const Rela = null();
    let mut rela_count = 0;

    let mut global_offset_table = null_mut();
    let mut symbol_table_pointer = null();
    for i in dynamic_array_iter {
        match i.d_tag {
            DT_RELA => rela_pointer = unsafe { base.byte_add(i.d_un.d_ptr.addr()) } as *const Rela,
            DT_RELASZ => {
                rela_count = unsafe { i.d_un.d_val } / core::mem::size_of::<Rela>();
            }
            #[cfg(debug_assertions)]
            DT_RELAENT => syscall_assert!(unsafe { i.d_un.d_val } as usize == size_of::<Rela>()),
            // other stuff we may need:
            DT_PLTGOT => global_offset_table = unsafe { base.byte_add(i.d_un.d_ptr.addr()) },
            DT_SYMTAB => {
                symbol_table_pointer =
                    unsafe { base.byte_add(i.d_un.d_ptr.addr()) } as *const Symbol
            }
            #[cfg(debug_assertions)]
            DT_SYMENT => syscall_assert!(unsafe { i.d_un.d_val } as usize == size_of::<Symbol>()),
            _ => (),
        }
    }
    syscall_assert!(!global_offset_table.is_null());
    syscall_assert!(!symbol_table_pointer.is_null());

    let base_address = base.addr();
    let global_offset_table_address = global_offset_table.addr();

    // Variables in relocation formulae:
    // - A(rela.r_addend): This is the addend used to compute the value of the relocatable field.
    // - B(base_address): This is the base address at which a shared object has been loaded into memory during execution.
    // - G(??): This is the offset into the global offset table at which the address of the relocation entry’s symbol will reside during execution.
    // - GOT(global_offset_table_address): This is the address of the global offset table.
    // - L(??): ??
    // - P(relocate_address): This is the address of the storage unit being relocated.
    // - S(symbol.st_value): This is the value of the symbol table entry indexed at `rela.r_sym()`.
    // - Z(??): ??

    // x86_64 relocation types:
    /// None
    const R_X86_64_NONE: u32 = 0;
    /// S + A
    const R_X86_64_64: u32 = 1;
    /// S + A - P
    const R_X86_64_PC32: u32 = 2;
    /// G + A
    const R_X86_64_GOT32: u32 = 3;
    /// L + A - P
    const R_X86_64_PLT32: u32 = 4;
    /// Copy directly from shared object.
    const R_X86_64_COPY: u32 = 5;
    /// S
    const R_X86_64_GLOB_DAT: u32 = 6;
    /// S
    const R_X86_64_JUMP_SLOT: u32 = 7;
    /// B + A
    const R_X86_64_RELATIVE: u32 = 8;
    /// G + GOT + A - P
    const R_X86_64_GOTPCREL: u32 = 9;
    /// S + A
    const R_X86_64_32: u32 = 10;
    /// S + A
    const R_X86_64_32S: u32 = 11;
    /// S + A
    const R_X86_64_16: u32 = 12;
    /// S + A - P
    const R_X86_64_PC16: u32 = 13;
    /// S + A
    const R_X86_64_8: u32 = 14;
    /// S + A - P
    const R_X86_64_PC8: u32 = 15;
    /// S + A - P
    const R_X86_64_PC64: u32 = 24;
    /// S + A - GOT
    const R_X86_64_GOTOFF64: u32 = 25;
    /// GOT + A - P
    const R_X86_64_GOTPC32: u32 = 26;
    /// Z + A
    const R_X86_64_SIZE32: u32 = 32;
    /// Z + A
    const R_X86_64_SIZE64: u32 = 33;
    // Yeah that's a lot of them...

    for rela in (0..rela_count).map(|i| unsafe { *rela_pointer.add(i) }) {
        let relocate_address = rela.r_offset.wrapping_add(base_address);
        let symbol = &*symbol_table_pointer.add(rela.r_sym() as usize);
        // TODO: clean this
        match rela.r_type() {
            R_X86_64_64 => {
                let relocate_value = symbol.st_value.wrapping_add_signed(rela.r_addend);
                asm!(
                    "mov qword ptr [{}], {}",
                    in(reg) relocate_address,
                    in(reg) relocate_value,
                    options(nostack, preserves_flags),
                );
            }
            R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                asm!(
                    "mov qword ptr [{}], {}",
                    in(reg) relocate_address,
                    in(reg) symbol.st_value,
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
            _ => (),
            // _ => {
            //     syscall_assert!(false, "unsupported relocation");
            // }
        }
    }
}

// https://en.wikipedia.org/wiki/Exit_(system_call)
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
    result
}
