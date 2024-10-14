use core::ptr::null;

use crate::{
    arch::relocation_store,
    elf::{
        dynamic_array::{DynamicArrayItem, DynamicArrayIter, DT_RELA, DT_RELASZ, DT_TEXTREL},
        program_header::{ElfProgramHeader, PT_DYNAMIC},
        relocate::{ElfRela, R_RELATIVE},
    },
    linux::io_macros::*,
};

pub(crate) fn relocate_linker(program_header_table: &[ElfProgramHeader], load_bias: usize) {
    syscall_debug_println!("gourmand relocating self...");
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
