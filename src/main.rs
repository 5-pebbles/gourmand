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

#[cfg_attr(target_arch = "x86_64", path = "arch/x86_64.rs")]
mod arch;

mod cli;
mod elf;
mod io_macros;
mod linux;

use elf::{
    dynamic_array::{
        DynamicArrayItem, DynamicArrayIter, DT_RELA, DT_RELAENT, DT_RELASZ, DT_STRTAB, DT_SYMENT,
        DT_SYMTAB,
    },
    header::{ElfHeader, ET_DYN},
    program_header::{ProgramHeader, PT_DYNAMIC, PT_TLS},
    relocate::Rela,
    symbol::Symbol,
};
use io_macros::*;
use linux::{
    auxiliary_vector::{
        AuxiliaryVectorIter, AT_BASE, AT_ENTRY, AT_PAGE_SIZE, AT_PHDR, AT_PHENT, AT_PHNUM,
        AT_RANDOM,
    },
    environment_variables::EnvironmentIter,
};

// This is where the magic happens, it's called by the architecture specific _start and returns the entry address when everything is set up:
pub(crate) unsafe fn rust_main(stack_pointer: *const usize) -> usize {
    // Check that `stack_pointer` is where we expect it to be.
    syscall_debug_assert!(stack_pointer != core::ptr::null_mut());
    syscall_debug_assert!(stack_pointer.addr() & 0b1111 == 0);

    let argument_count = *stack_pointer as usize;
    let argument_pointer = stack_pointer.add(1) as *mut *mut u8;
    syscall_debug_assert!((*argument_pointer.add(argument_count)).is_null());

    let environment_iter = EnvironmentIter::from_stack_pointer(stack_pointer);
    let auxiliary_vector_iter = AuxiliaryVectorIter::from_environment_iter(environment_iter);

    // Auxilary Vector:
    let (mut base, mut entry, mut page_size) = (null(), null(), 0);
    let mut pseudorandom_bytes: *const [u8; 16] = null_mut();
    // NOTE: The program headers in the auxiliary vector belong to the executable, not us.
    let (mut program_header_pointer, mut program_header_count) = (null(), 0);
    for item in auxiliary_vector_iter {
        match item.a_type {
            AT_BASE => base = item.a_un.a_ptr,
            AT_ENTRY => entry = item.a_un.a_ptr,
            AT_PAGE_SIZE => page_size = item.a_un.a_val,
            AT_RANDOM => pseudorandom_bytes = item.a_un.a_ptr as *const [u8; 16],
            AT_PHDR => program_header_pointer = item.a_un.a_ptr as *const ProgramHeader,
            AT_PHNUM => program_header_count = item.a_un.a_val,
            #[cfg(debug_assertions)]
            AT_PHENT => syscall_assert!(item.a_un.a_val == size_of::<ProgramHeader>()),
            _ => (),
        }
    }

    if base.is_null() {
        // This means we are a static pie (position-independent-executable) -  probably called as ./miros
        cli::run_cli();
    }

    // ELf Header:
    let header = &*(base as *const ElfHeader);
    syscall_debug_assert!(header.e_type == ET_DYN);
    syscall_debug_assert!(header.e_phentsize == size_of::<ProgramHeader>() as u16);

    // Program Headers:
    let program_header_table = slice::from_raw_parts(
        base.byte_add(header.e_phoff) as *const ProgramHeader,
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
    let (dynamic_header, tls_program_header) =
        (dynamic_header.unwrap(), tls_program_header.unwrap());

    // Dynamic Arrary:
    let dynamic_array_pointer = base.byte_add(dynamic_header.p_vaddr) as *const DynamicArrayItem;

    let mut rela_pointer: *const Rela = null();
    let mut rela_count = 0;

    let mut symbol_table_pointer = null();
    let mut string_table_pointer = null();
    for item in DynamicArrayIter::new(dynamic_array_pointer) {
        match item.d_tag {
            DT_RELA => rela_pointer = base.byte_add(item.d_un.d_ptr.addr()) as *const Rela,
            DT_RELASZ => {
                rela_count = item.d_un.d_val / core::mem::size_of::<Rela>();
            }
            #[cfg(debug_assertions)]
            DT_RELAENT => {
                syscall_assert!(item.d_un.d_val as usize == size_of::<Rela>())
            }
            DT_SYMTAB => {
                symbol_table_pointer = base.byte_add(item.d_un.d_ptr.addr()) as *const Symbol
            }
            DT_STRTAB => string_table_pointer = base.byte_add(item.d_un.d_ptr.addr()) as *const u8,
            #[cfg(debug_assertions)]
            DT_SYMENT => {
                syscall_assert!(item.d_un.d_val as usize == size_of::<Symbol>())
            }
            _ => (),
        }
    }
    syscall_assert!(!symbol_table_pointer.is_null());
    syscall_assert!(!string_table_pointer.is_null());

    // Relocations:
    let rela = slice::from_raw_parts(rela_pointer, rela_count);
    arch::relocate(base, symbol_table_pointer, rela);

    // Thread Local Storage:
    arch::init_thread_local_storage(base, *tls_program_header, pseudorandom_bytes);

    syscall_debug_assert!(page_size.is_power_of_two());
    syscall_debug_assert!(base.addr() & (page_size - 1) == 0);

    let mybox = Box::new(0);

    arch::exit(0);
}
