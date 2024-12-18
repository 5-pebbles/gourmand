use crate::{arch, io_macros::*};

pub fn run_cli() -> ! {
    // write(1, concat!(env!("CARGO_PKG_DESCRIPTION"), "\n"));
    // write(1, bold!(underline!("Usage:"), " miros"), " <BINARY_PATH>\n");
    // write(1, "This doesn't work yet");
    arch::exit::exit(0);
}
