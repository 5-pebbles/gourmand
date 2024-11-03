use core::{ffi::c_void, ptr::null_mut};
use core::{cmp::max, mem::offset_of};

use crate::{
    elf::program_header::{ProgramHeader, PT_TLS},
    linux::io_macros::*,
};

pub(crate) struct TLSTemplateInfo {
    pub pointer: *mut c_void,
    pub image_size: usize,
    pub template_size: usize,
    pub template_alignment
}

pub(crate) fn initialize_tls(shared_object: &SharedObject) {
    let mut tls_program_header = None;
    shared_object
        .program_header
        .into_iter()
        .for_each(|header| match header.p_type {
            PT_TLS => tls_program_header = Some(header),
            _ => (),
        });

    
    if let Some(header) = tls_program_header {
        header
    } else {
        
    }

    unsafe {
        main_tls.size += main_tls
            .size
            .wrapping_neg()
            .wrapping_sub(main_tls.image.addr())
            & (main_tls.align - 1);

        // Thread Local Storage [above Thread Pointer]:                 |---------------------|
        //                                                              |    ...tls-offset    |
        //                                                              |---------------------|
        //                       |-> |----------------------------| <-- |    tls-offset[-2]   |
        // |----------|          |   |   ...Static TLS Blocks     |     |---------------------|
        // |   GENt   | <-|   |--|-> |----------------------------| <-- |    tls-offset[-1]   |
        // |----------|   |   |  |   |      Static TLS Block      |     |---------------------|
        // |  DTV[1]  | --|---|  |   |----------------------------| <-- | Thread Pointer (TP) |
        // |----------|   |------|-- | Thread Control Block (TCB) |     |---------------------|
        // |  DTV[2]  |----------|   |----------------------------|
        // |----------|
        // |  DTV[3]  | --------> |------------------------------|
        // |----------|           | Dynamically−loaded TLS Block |
        // |  DTV[4]  | --|       |------------------------------|
        // |----------|   |
        // |  DTV...  |   |-----> |------------------------------|
        // |----------|           | Dynamically−loaded TLS Block |
        //                        |------------------------------|
        #[cfg(any(target_arch = "x86_64"))]
        {
            main_tls.offset = main_tls.size;
        }
        // Thread Local Storage [below Thread Pointer]:             |---------------------|
        //                    |----------------------------| <----- | Thread Pointer (TP) |
        // |----------| <---- | Dynamic Thread Vec Pointer |        |---------------------|
        // |   GENt   |       |----------------------------|   |--- |    tls-offset[1]    |
        // |----------|       | Thread Control Block (TCB) |   |    |---------------------|
        // |  DTV[1]  | ----> |----------------------------| <-| |- |    tls-offset[2]    |
        // |----------|       |    Static TLS Blocks...    |     |  |---------------------|
        // |  DTV[2]  | ----> |----------------------------| <---|  |     tls-offset...   |
        // |----------|                                             |---------------------|
        // |  DTV[3]  | --------> |------------------------------|
        // |----------|           | Dynamically−loaded TLS Block |
        // |  DTV[4]  | --|       |------------------------------|
        // |----------|   |
        // |  DTV...  |   |-----> |------------------------------|
        // |----------|           | Dynamically−loaded TLS Block |
        //                        |------------------------------|
        #[cfg(any())]
        {
            // TODO: I don't have a way of testing this, and I am not comfortable writing code that I can't test.
            syscall_assert!(false, "TODO");
        }

        let min_tls_align = offset_of!(TlsModule, align);
        main_tls.align = max(main_tls.align, min_tls_align);
    }
}
