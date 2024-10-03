use core::ptr::null;

use crate::{
    arch::relocation_store,
    elf::{
        dynamic_array::{DynamicArrayItem, DynamicArrayIter, DT_RELA, DT_RELASZ, DT_TEXTREL},
        header::{self, ElfHeader},
        program_header::{ElfProgramHeaderTable, PT_DYNAMIC, PT_LOAD},
        relocate::{ElfRela, R_RELATIVE},
    },
    linux::auxiliary_vector::{AuxiliaryVectorIter, AT_BASE, AT_ENTRY, AT_PAGE_SIZE},
    utils::no_std_debug_assert,
};

pub(crate) fn relocate_linker(auxiliary_iterator: AuxiliaryVectorIter) {
    // TODO: This needs to be split up... real bad...
    no_std_debug_assert!(auxiliary_iterator
        .clone()
        .any(|value| value.a_type == AT_PAGE_SIZE));
    no_std_debug_assert!(auxiliary_iterator
        .clone()
        .any(|value| value.a_type == AT_BASE));
    no_std_debug_assert!(auxiliary_iterator
        .clone()
        .any(|value| value.a_type == AT_ENTRY));

    let (mut base, mut entry, mut page_size) = (0, 0, 0);

    auxiliary_iterator.for_each(|value| match value.a_type {
        AT_BASE => base = value.a_val,
        AT_ENTRY => entry = value.a_val,
        AT_PAGE_SIZE => page_size = value.a_val,
        _ => (),
    });

    let header = unsafe { &*(base as *const ElfHeader) };
    no_std_debug_assert!(header.e_type == header::ET_DYN);

    let program_header_table = ElfProgramHeaderTable::new(base, header.e_phoff, header.e_phnum);
    let load_bias = base
        - program_header_table
            .iter()
            .find(|f| f.p_type == PT_LOAD)
            .unwrap()
            .p_vaddr;

    let dynamic_header = program_header_table
        .into_iter()
        .find(|h| h.p_type == PT_DYNAMIC)
        .unwrap();

    let dynamic_array = DynamicArrayIter::new(
        dynamic_header.p_vaddr.wrapping_add(load_bias) as *const DynamicArrayItem
    );

    no_std_debug_assert!(dynamic_array.clone().any(|i| i.d_tag == DT_RELA));
    // TODO: How do you handle these cases?
    no_std_debug_assert!(!dynamic_array.clone().any(|i| i.d_tag == DT_TEXTREL));

    let mut rela_pointer: *const ElfRela = null();
    let mut rela_length = 0;
    dynamic_array.for_each(|i| match i.d_tag {
        DT_RELA => rela_pointer = (load_bias + unsafe { i.d_un.d_ptr }) as *const ElfRela,
        DT_RELASZ => {
            rela_length = unsafe { i.d_un.d_val } / core::mem::size_of::<ElfRela>();
        }
        _ => (),
    });

    for rela in (0..rela_length).map(|i| unsafe { *rela_pointer.add(i) }) {
        let relocate_address = rela.r_offset.wrapping_add(load_bias);

        match rela.r_type {
            R_RELATIVE => {
                let relocate_value = rela.r_addend.wrapping_add(load_bias);
                unsafe { relocation_store(relocate_address, relocate_value) };
            }
            _ => (),
        }
    }
}
