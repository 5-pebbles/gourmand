use crate::{
    arch::{exit, write},
    linux::{
        auxiliary_vector::{AuxiliaryVectorItem, AuxiliaryVectorIter},
        environment_iterator::EnvironmentIterator,
        relocate::relocate_linker,
    },
    utils::{self, no_std_debug_assert},
};

pub unsafe fn rust_start(stack_pointer: *const usize) -> usize {
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
    let environment_iterator = EnvironmentIterator::new(argument_pointer.add(argument_count + 1));

    no_std_debug_assert!((*argument_pointer.add(argument_count)).is_null());

    let auxiliary_iterator = AuxiliaryVectorIter::from_environment_iterator(environment_iterator);

    relocate_linker(auxiliary_iterator);

    exit(0)
}
