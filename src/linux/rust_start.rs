use core::{
    arch::asm,
    ffi::c_void,
    ptr::{null, null_mut},
    slice,
};

use crate::{
    arch::{exit, relocate},
    elf::{
        dynamic_array::{DynamicArrayItem, DynamicArrayIter},
        header::{ElfHeader, ET_DYN},
        program_header::{ProgramHeader, PT_DYNAMIC},
    },
    global_allocator,
    linux::{
        auxiliary_vector::{AuxiliaryVectorIter, AT_BASE, AT_ENTRY, AT_PAGE_SIZE},
        environment_variables::EnvironmentIter,
        io_macros::*,
        shared_object::SharedObject,
        thread_local_storage::initialize_tls,
    },
};

pub(crate) unsafe fn rust_start(stack_pointer: *const usize) -> usize {
    // Check that `stack_pointer` is where we expect it to be.
    syscall_debug_assert!(stack_pointer != core::ptr::null_mut());
    syscall_debug_assert!(stack_pointer.addr() & 0b1111 == 0);

    let argument_count = *stack_pointer as usize;
    let argument_pointer = stack_pointer.add(1) as *mut *mut u8;
    syscall_debug_assert!((*argument_pointer.add(argument_count)).is_null());

    let environment_iter = EnvironmentIter::from_stack_pointer(stack_pointer);
    let auxiliary_vector_iter = AuxiliaryVectorIter::from_environment_iter(environment_iter);

    let (mut base, mut entry, mut page_size) = (null(), null(), 0);
    auxiliary_vector_iter.for_each(|value| match value.a_type {
        AT_BASE => base = value.a_val,
        AT_ENTRY => entry = value.a_val,
        AT_PAGE_SIZE => page_size = value.a_val.addr(),
        _ => (),
    });

    if base.is_null() {
        // This means we are a static pie (position-independent-executable) -  probably called as ./libgourmand.so
        syscall_println!(concat!(env!("CARGO_PKG_DESCRIPTION"), "\n"));
        syscall_println!(bold!(underline!("Usage:"), " gourmand"), " <BINARY_PATH>\n");
        syscall_println!("This doesn't work yet");
        exit(0);
    }

    let shared_object = SharedObject::new(base);

    relocate(&shared_object);

    syscall_debug_assert!(page_size.is_power_of_two());
    syscall_debug_assert!(base.addr() & (page_size - 1) == 0);

    global_allocator::ALLOCATOR.initialize(page_size);
    let mybox = Box::new(0);

    // TODO: init got
    exit(0);
}
