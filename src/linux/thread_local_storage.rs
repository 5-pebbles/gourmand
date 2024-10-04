use core::{ffi::c_void, ptr::null_mut};

use crate::elf::program_header::ElfProgramHeader;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TlsModule {
    pub next: *mut TlsModule,
    pub image: *mut c_void,
    pub len: usize,
    pub size: usize,
    pub align: usize,
    pub offset: usize,
}

impl TlsModule {
    pub(crate) const fn const_default() -> Self {
        Self {
            next: null_mut(),
            image: null_mut(),
            len: 0,
            size: 0,
            align: 0,
            offset: 0,
        }
    }
}

#[no_mangle]
#[used]
pub static mut main_tls: TlsModule = TlsModule::const_default();

pub(crate) fn initialize_tls(program_header: &[ElfProgramHeader]) {
    // TODO
}
