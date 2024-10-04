use core::slice;

use crate::{
    arch::exit,
    elf::{
        header::{ElfHeader, ET_DYN},
        program_header::{ElfProgramHeader, ElfProgramHeaderTable, PT_LOAD},
    },
    linux::{
        auxiliary_vector::{AuxiliaryVectorIter, AT_BASE, AT_ENTRY, AT_PAGE_SIZE},
        environment_iterator::EnvironmentIterator,
        relocate::relocate_linker,
        thread_local_storage::initialize_tls,
    },
    utils::no_std_debug_assert,
};

pub(crate) unsafe fn rust_start(stack_pointer: *const usize) -> usize {
    // Stack layout:
    // |---------------------|
    // | arg_count           |
    // |---------------------|
    // | arg_values...       |
    // |---------------------|
    // | null                |
    // |---------------------|
    // | env_pointers...     |
    // |---------------------|
    // | null                |
    // |---------------------|
    // | null                |
    // |---------------------|
    // | auxiliary_vector... |
    // |---------------------|
    // | null                |
    // |---------------------|
    // | ...                 |
    // |---------------------|
    // Check that `stack_pointer` is where we expect it to be.
    no_std_debug_assert!(stack_pointer != core::ptr::null_mut());
    no_std_debug_assert!(stack_pointer.addr() & 0b1111 == 0);

    let argument_count = *stack_pointer as usize;
    let argument_pointer = stack_pointer.add(1) as *mut *mut u8;
    no_std_debug_assert!((*argument_pointer.add(argument_count)).is_null());

    let environment_iterator = EnvironmentIterator::new(argument_pointer.add(argument_count + 1));

    let auxiliary_vector = AuxiliaryVectorIter::from_environment_iterator(environment_iterator);
    no_std_debug_assert!(auxiliary_vector
        .clone()
        .any(|value| value.a_type == AT_PAGE_SIZE));
    no_std_debug_assert!(auxiliary_vector
        .clone()
        .any(|value| value.a_type == AT_BASE));
    no_std_debug_assert!(auxiliary_vector
        .clone()
        .any(|value| value.a_type == AT_ENTRY));

    let (mut base, mut entry, mut page_size) = (0, 0, 0);
    auxiliary_vector.for_each(|value| match value.a_type {
        AT_BASE => base = value.a_val,
        AT_ENTRY => entry = value.a_val,
        AT_PAGE_SIZE => page_size = value.a_val,
        _ => (),
    });

    let header = unsafe { &*(base as *const ElfHeader) };
    no_std_debug_assert!(header.e_type == ET_DYN);

    let program_header_table = ElfProgramHeaderTable::new(base, header.e_phoff, header.e_phnum);
    let load_bias = base
        - program_header_table
            .iter()
            .find(|f| f.p_type == PT_LOAD)
            .unwrap()
            .p_vaddr;

    let program_header_table = slice::from_raw_parts(
        (base + header.e_phoff) as *const ElfProgramHeader,
        header.e_phnum as usize,
    );
    relocate_linker(program_header_table, load_bias);

    initialize_tls(program_header_table);

    exit(0)
}
