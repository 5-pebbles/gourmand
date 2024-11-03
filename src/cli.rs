use crate::{arch, io_macros::*};

pub(crate) fn run_cli() -> ! {
    syscall_println!(concat!(env!("CARGO_PKG_DESCRIPTION"), "\n"));
    syscall_println!(bold!(underline!("Usage:"), " miros"), " <BINARY_PATH>\n");
    syscall_println!("This doesn't work yet");
    arch::exit(0);
}
