use core::{ffi::c_void, ptr::null_mut};

use crate::elf::program_header::{ElfProgramHeader, PT_TLS};

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

pub(crate) fn initialize_tls(program_header: &[ElfProgramHeader], load_bias: usize) {
    let mut tls_program_header = None;
    program_header
        .into_iter()
        .for_each(|header| match header.p_type {
            PT_TLS => tls_program_header = Some(header),
            _ => (),
        });

    if let Some(header) = tls_program_header {
        unsafe {
            main_tls.image = (load_bias + header.p_vaddr) as *mut c_void;
            main_tls.len = header.p_filesz;
            main_tls.size = header.p_memsz;
            main_tls.align = header.p_align;
            // TODO libc global
        }
    }
    // TODO
}
