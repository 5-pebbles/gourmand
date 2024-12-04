use core::arch::{asm, naked_asm};

use crate::{io_macros::*, syscall_debug_assert};

pub mod exit;
pub mod io;
pub mod mmap;
pub mod relocation;
pub mod thread_pointer;

#[naked]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!("mov rdi, rsp",
        "and rsp, -16", // !0b1111
        "call {}",
        "mov rdx, 0",
        "jmp rax",
        sym crate::rust_main,
    );
}
