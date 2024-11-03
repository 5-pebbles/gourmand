use core::{
    ffi::c_void,
    ptr::{null, null_mut},
    slice,
};

use crate::{
    elf::{
        dynamic_array::{
            DynamicArrayItem, DynamicArrayIter, DT_NEEDED, DT_PLTGOT, DT_RELA, DT_RELAENT,
            DT_RELASZ, DT_STRTAB, DT_SYMENT, DT_SYMTAB,
        },
        header::{ElfHeader, ET_DYN},
        program_header::{ProgramHeader, PT_DYNAMIC, PT_TLS},
        relocate::Rela,
        symbol::Symbol,
    },
    linux::io_macros::*,
};

pub(crate) struct SharedObject {
    pub load_bias: *const c_void,
    pub header: &'static ElfHeader,
    pub rela: &'static [Rela],
    pub program_header_table: &'static [ProgramHeader],
    pub dynamic_array_pointer: *const DynamicArrayItem,
    pub tls_program_header: Option<&'static ProgramHeader>,
    pub symbol_table_pointer: *const Symbol,
    pub string_table_pointer: *const u8,
    // pub global_offset_table_pointer: *mut usize,
}

impl SharedObject {
    pub unsafe fn new(load_bias: *const c_void) -> Self {
        syscall_debug_assert!(!load_bias.is_null());
        let header = &*(load_bias as *const ElfHeader);
        syscall_debug_assert!(header.e_type == ET_DYN);
        syscall_debug_assert!(header.e_phentsize == size_of::<ProgramHeader>() as u16);

        let program_header_table = slice::from_raw_parts(
            load_bias.byte_add(header.e_phoff) as *const ProgramHeader,
            header.e_phnum as usize,
        );

        let (mut dynamic_header, mut tls_program_header) = (None, None);
        for header in program_header_table {
            match header.p_type {
                PT_DYNAMIC => dynamic_header = Some(header),
                PT_TLS => tls_program_header = Some(header),
                _ => (),
            }
        }

        let dynamic_array_pointer =
            load_bias.byte_add(dynamic_header.unwrap().p_vaddr) as *const DynamicArrayItem;

        let mut rela_pointer: *const Rela = null();
        let mut rela_count = 0;

        let mut global_offset_table_pointer = null_mut();
        let mut symbol_table_pointer = null();
        let mut string_table_pointer = null();
        for item in DynamicArrayIter::new(dynamic_array_pointer) {
            match item.d_tag {
                DT_RELA => rela_pointer = load_bias.byte_add(item.d_un.d_ptr.addr()) as *const Rela,
                DT_RELASZ => {
                    rela_count = item.d_un.d_val / core::mem::size_of::<Rela>();
                }
                #[cfg(debug_assertions)]
                DT_RELAENT => {
                    syscall_assert!(item.d_un.d_val as usize == size_of::<Rela>())
                }
                // other stuff we may need:
                DT_PLTGOT => {
                    global_offset_table_pointer =
                        load_bias.byte_add(item.d_un.d_ptr.addr()) as *mut usize
                }
                DT_SYMTAB => {
                    symbol_table_pointer =
                        load_bias.byte_add(item.d_un.d_ptr.addr()) as *const Symbol
                }
                DT_STRTAB => {
                    string_table_pointer = load_bias.byte_add(item.d_un.d_ptr.addr()) as *const u8
                }
                #[cfg(debug_assertions)]
                DT_SYMENT => {
                    syscall_assert!(item.d_un.d_val as usize == size_of::<Symbol>())
                }
                _ => (),
            }
        }
        syscall_assert!(global_offset_table_pointer.is_null());
        syscall_assert!(!symbol_table_pointer.is_null());

        let rela = slice::from_raw_parts(rela_pointer, rela_count);

        Self {
            load_bias,
            header,
            program_header_table,
            dynamic_array_pointer,
            tls_program_header,
            rela,
            symbol_table_pointer,
            string_table_pointer,
            // global_offset_table_pointer,
        }
    }
}
