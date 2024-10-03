use core::arch::asm;

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

#[inline]
pub(super) unsafe fn relocation_store(ptr: usize, value: usize) {
    asm!(
        "mov [{}], {}",
        in(reg) ptr,
        in(reg) value,
        options(nostack, preserves_flags),
    );
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
