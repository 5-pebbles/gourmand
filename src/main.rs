#![feature(strict_provenance)]
#![feature(impl_trait_in_assoc_type)]
#![feature(naked_functions)]
#![feature(ptr_as_ref_unchecked)]
#![no_main]
#![allow(dead_code)]

use core::{
    ptr::{null, null_mut},
    slice,
};
use std::{collections::HashMap, fs::File, path::Path};

#[cfg_attr(target_arch = "x86_64", path = "arch/x86_64/mod.rs")]
mod arch;

mod cli;
mod elf;
mod io_macros;
mod linux;
mod shared_object;

use elf::program_header::ProgramHeader;
use io_macros::*;
use linux::{
    auxiliary_vector::{
        AuxiliaryVectorIter, AT_BASE, AT_ENTRY, AT_PAGE_SIZE, AT_PHDR, AT_PHENT, AT_PHNUM,
        AT_RANDOM,
    },
    environment_variables::EnvironmentIter,
};
use shared_object::SharedObject;

// This is where the magic happens, it's called by the architecture specific _start and returns the entry address when everything is set up:
pub(crate) unsafe fn rust_main(stack_pointer: *mut usize) -> usize {
    // Check that `stack_pointer` is where we expect it to be.
    syscall_debug_assert!(stack_pointer != core::ptr::null_mut());
    syscall_debug_assert!(stack_pointer.addr() & 0b1111 == 0);

    let argument_count = *stack_pointer as usize;
    let argument_pointer = stack_pointer.add(1) as *mut *mut u8;
    syscall_debug_assert!((*argument_pointer.add(argument_count)).is_null());

    let environment_vector = EnvironmentIter::from_stack_pointer(stack_pointer);
    let auxiliary_vector = AuxiliaryVectorIter::from_environment_iter(environment_vector);

    // Auxilary Vector:
    let (mut base, mut entry, mut page_size) = (null(), null(), 0);
    let mut pseudorandom_bytes: *const [u8; 16] = null_mut();
    // NOTE: The program headers in the auxiliary vector belong to the executable, not us.
    let (mut program_header_pointer, mut program_header_count) = (null(), 0);
    for value in auxiliary_vector {
        match value.a_type {
            AT_BASE => base = value.a_un.a_ptr,
            AT_ENTRY => entry = value.a_un.a_ptr,
            AT_PAGE_SIZE => page_size = value.a_un.a_val,
            AT_RANDOM => pseudorandom_bytes = value.a_un.a_ptr as *const [u8; 16],
            // Executable Stuff:
            AT_PHDR => program_header_pointer = value.a_un.a_ptr as *const ProgramHeader,
            AT_PHNUM => program_header_count = value.a_un.a_val,
            #[cfg(debug_assertions)]
            AT_PHENT => syscall_assert!(value.a_un.a_val == size_of::<ProgramHeader>()),
            _ => (),
        }
    }

    if base.is_null() {
        // This means we are a static pie (position-independent-executable) -  probably called as ./miros
        cli::run_cli();
    }

    // Relocate ourselves and initialize thread local storage:
    let miros_shared_object = SharedObject::from_load_address(base);
    arch::relocate(&miros_shared_object);
    arch::init_thread_local_storage(&miros_shared_object, pseudorandom_bytes);

    syscall_debug_assert!(page_size.is_power_of_two());
    syscall_debug_assert!(base.addr() & (page_size - 1) == 0);

    // NOTE: We can now use the Rust standard library.
    // Except for `format_args`... ¯\_(ツ)_/¯ idk

    let program_header_table =
        slice::from_raw_parts(program_header_pointer, program_header_count as usize);
    let shared_object = SharedObject::from_program_header_table(&program_header_table);

    let linked_shared_objects: HashMap<&'static str, SharedObject> = HashMap::new();
    for library in shared_object.libraries() {
        syscall_println!("Loading ", library);
        if linked_shared_objects.contains_key(library) {
            continue;
        }
    }

    arch::exit(0);
}
