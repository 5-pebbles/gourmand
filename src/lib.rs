#![feature(strict_provenance)]
#![feature(naked_functions)]
#![no_std]

#[cfg_attr(target_arch = "x86_64", path = "arch/x86_64.rs")]
mod arch;

mod linux;
mod utils;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
