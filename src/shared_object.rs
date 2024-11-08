use core::ptr::null;
use core::{ffi::c_void, slice};

use crate::elf::dynamic_array::{DynamicArrayItem, DT_NEEDED};
use crate::elf::program_header::PT_LOAD;
use crate::elf::symbol::SymbolTable;
use crate::{
    elf::{
        dynamic_array::{
            DynamicArrayIter, DT_RELA, DT_RELAENT, DT_RELASZ, DT_STRTAB, DT_SYMENT, DT_SYMTAB,
        },
        header::{ElfHeader, ET_DYN},
        program_header::{ProgramHeader, PT_DYNAMIC, PT_PHDR, PT_TLS},
        relocate::Rela,
        string_table::StringTable,
        symbol::Symbol,
    },
    syscall_assert, syscall_debug_assert,
};

pub(crate) struct RelocationInfo {
    pub rela: &'static [Rela],
}

pub(crate) struct ThreadLocalStoreInfo {
    pub program_header: &'static ProgramHeader,
}

pub(crate) struct SharedObject {
    pub base: *const c_void,
    pub relocations: RelocationInfo,
    pub tls: ThreadLocalStoreInfo,
    pub needed_libraries: Vec<usize>,
    pub symbol_table: SymbolTable,
    pub string_table: StringTable,
}

impl SharedObject {
    pub unsafe fn from_load_address(base: *const c_void) -> Self {
        // ELf Header:
        let header = &*(base as *const ElfHeader);
        syscall_debug_assert!(header.e_type == ET_DYN);
        syscall_debug_assert!(header.e_phentsize == size_of::<ProgramHeader>() as u16);

        // Program Headers:
        let program_header_table = slice::from_raw_parts(
            base.byte_add(header.e_phoff) as *const ProgramHeader,
            header.e_phnum as usize,
        );

        Self::from_program_header_table(program_header_table)
    }

    pub unsafe fn from_program_header_table(
        program_header_table: &'static [ProgramHeader],
    ) -> Self {
        let (mut base, mut dynamic_header, mut tls_program_header) = (null(), null(), null());
        for header in program_header_table {
            match header.p_type {
                PT_PHDR => {
                    base = program_header_table.as_ptr().byte_sub(header.p_vaddr) as *const c_void
                }
                PT_DYNAMIC => dynamic_header = header,
                PT_TLS => tls_program_header = header,
                _ => (),
            }
        }

        // Dynamic Arrary:
        let dynamic_array = DynamicArrayIter::new(
            base.byte_add((*dynamic_header).p_vaddr) as *const DynamicArrayItem
        );

        let mut rela_pointer: *const Rela = null();
        let mut rela_count = 0;

        let mut symbol_table_pointer: *const Symbol = null();
        let mut string_table_pointer: *const u8 = null();
        let mut needed_libraries = Vec::new();
        for item in dynamic_array {
            match item.d_tag {
                DT_NEEDED => needed_libraries.push(item.d_un.d_val),
                DT_RELA => rela_pointer = base.byte_add(item.d_un.d_ptr.addr()) as *const Rela,
                DT_RELASZ => {
                    rela_count = item.d_un.d_val / core::mem::size_of::<Rela>();
                }
                #[cfg(debug_assertions)]
                DT_RELAENT => {
                    syscall_assert!(item.d_un.d_val as usize == size_of::<Rela>())
                }
                // Tables:
                DT_SYMTAB => {
                    symbol_table_pointer = base.byte_add(item.d_un.d_ptr.addr()) as *const Symbol
                }
                DT_STRTAB => {
                    string_table_pointer = base.byte_add(item.d_un.d_ptr.addr()) as *const u8
                }
                #[cfg(debug_assertions)]
                DT_SYMENT => {
                    syscall_assert!(item.d_un.d_val as usize == size_of::<Symbol>())
                }
                _ => (),
            }
        }
        syscall_debug_assert!(!string_table_pointer.is_null());

        syscall_debug_assert!(rela_pointer != null());
        let rela_slice = slice::from_raw_parts(rela_pointer, rela_count);

        Self {
            base,
            relocations: RelocationInfo { rela: rela_slice },
            tls: ThreadLocalStoreInfo {
                program_header: &*tls_program_header,
            },
            needed_libraries,
            symbol_table: SymbolTable::new(symbol_table_pointer),
            string_table: StringTable::new(string_table_pointer),
        }
    }

    pub unsafe fn libraries(&self) -> Vec<&'static str> {
        self.needed_libraries
            .iter()
            .map(|&index| self.string_table.get(index as usize))
            .collect()
    }
}
