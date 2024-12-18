use std::arch::asm;

use crate::syscall_debug_assert;

// Protection flags:
pub const PROT_NONE: usize = 0x0;
pub const PROT_READ: usize = 0x1;
pub const PROT_WRITE: usize = 0x2;
pub const PROT_EXEC: usize = 0x4;
pub const PROT_GROWSDOWN: isize = 0x01000000;
pub const PROT_GROWSUP: isize = 0x02000000;

// MAP flags:
pub const MAP_FILE: usize = 0x0;
pub const MAP_SHARED: usize = 0x1;
pub const MAP_PRIVATE: usize = 0x2;
pub const MAP_FIXED: usize = 0x10;
pub const MAP_ANONYMOUS: usize = 0x20;

// #[inline(always)]
pub unsafe fn mmap(
    pointer: *mut (),
    size: usize,
    protection_flags: usize,
    map_flags: usize,
    file_descriptor: isize,
    offset: usize,
) -> *mut () {
    const MMAP: usize = 9; // I am like 80% sure this is the right system call... :)

    let mut result: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") MMAP => result,
            in("rdi") pointer,
            in("rsi") size,
            in("rdx") protection_flags,
            in("r10") map_flags,
            in("r8") file_descriptor,
            in("r9") offset,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    syscall_debug_assert!(result >= 0);
    result as *mut ()
}

#[inline(always)]
pub unsafe fn munmap(pointer: *mut (), size: usize) {
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
