// TODO: Fix
// NOTE: I no longer remember what I was supposed to `fix` but I bet it must have been important.
use core::slice;
use std::ptr::null_mut;

use crate::{
    arch::{mmap, thread_pointer},
    elf::{
        program_header::{ProgramHeader, PT_TLS},
        thread_local_storage::{DtvItem, ThreadControlBlock},
    },
    syscall_debug_assert,
    utils::round_up_to_boundary,
};

pub(crate) type LachesisItem = (*const (), ProgramHeader);

// NOTE: Lachesis is the middle of the Three Fates and is responsible for measuring each thread of life, then choosing its destiny. The name is first used by dryad, an (incomplete) dynamic linker written in rust.
pub(crate) struct Lachesis {
    pseudorandom_bytes: *const [u8; 16],
    tls_program_headers: Vec<LachesisItem>,
}

impl Lachesis {
    pub fn new(pseudorandom_bytes: *const [u8; 16]) -> Self {
        Self {
            pseudorandom_bytes,
            tls_program_headers: Vec::new(),
        }
    }

    pub fn push(&mut self, base: *const (), tls_program_header: ProgramHeader) -> usize {
        syscall_debug_assert!(tls_program_header.p_type == PT_TLS);
        self.tls_program_headers.push((base, tls_program_header));
        self.tls_program_headers.len()
    }

    pub fn allocate(self) {
        // Thread Local Storage [before Thread Pointer]:
        //                           |----------------------------|         |---------------------|
        // |----------|              |       TCB Alignment        |         |    ...tls-offset    |
        // |  length  |          |-> |----------------------------| <----|  |---------------------|
        // |----------| <-|      |   |  ...Static TLS Blocks      |      |- |    tls-offset[-2]   |
        // |   GENt   |   |   |--|-> |----------------------------| <-|     |---------------------|
        // |----------|   |   |  |   |      Static TLS Block      |   |---- |   tls-offset[-1]    |
        // |  DTV[1]  | --|---|  |   |----------------------------|         |---------------------|
        // |----------|   |      |   |    Static TLS Alignment    |   |---- | Thread Pointer (TP) |
        // |  DTV[2]  |---|------|   |----------------------------| <-|     |---------------------|
        // |----------|   |--------- | Thread Control Block (TCB) |
        // |  DTV[3]  | ----|        |----------------------------|
        // |----------|     |
        // |  DTV[4]  | -|  |   |------------------------------|
        // |----------|  |  |-> | Dynamically−loaded TLS Block |
        // |  DTV...  |  |      |------------------------------|
        // |----------|  |
        //               |      |------------------------------|
        //               |----> | Dynamically−loaded TLS Block |
        //                      |------------------------------|
        // NOTE: length is not part of the ABI but glibc uses it and there isn't really an ABI anyway...
        let mut tcb_align = align_of::<ThreadControlBlock>();
        let mut tls_blocks_size = 0;
        for (_, header) in &self.tls_program_headers {
            tls_blocks_size += round_up_to_boundary(header.p_memsz, header.p_align);
        }
        let tcb_size = size_of::<ThreadControlBlock>();

        // NOTE: this includes length and GENt (hence the +2).
        let dtv_total_len = self.tls_program_headers.len() + 2;
        let dtv_size = dtv_total_len * size_of::<DtvItem>();

        let required_size = tcb_align + tls_blocks_size + tcb_size + dtv_size;
        // We use system calls because the allocator itself may use TLS:
        unsafe {
            let tls_allocation_pointer = mmap::mmap(
                null_mut(),
                required_size,
                mmap::PROT_READ | mmap::PROT_WRITE,
                mmap::MAP_PRIVATE | mmap::MAP_ANONYMOUS,
                -1, // file descriptor (-1 for anonymous mapping)
                0,  // offset
            );
            syscall_debug_assert!(tls_allocation_pointer.addr() % tcb_align == 0);

            // Initialize the Dynamic Thread Vector (DTV):
            let mut dynamic_thread_vector = Vec::from_raw_parts(
                tls_allocation_pointer.byte_add(tcb_align + tls_blocks_size + tcb_size)
                    as *mut DtvItem,
                0,
                dtv_total_len,
            );
            dynamic_thread_vector.push(DtvItem {
                length: self.tls_program_headers.len(),
            });
            dynamic_thread_vector.push(DtvItem {
                generation_counter: 0,
            });

            let mut tls_block_pointer = tls_allocation_pointer.byte_add(tcb_align);
            for (base, header) in &self.tls_program_headers {
                // Align:
                tls_block_pointer =
                    round_up_to_boundary(tls_block_pointer.addr(), header.p_align) as *mut ();

                // Initialize the TLS data from template image:
                slice::from_raw_parts_mut(tls_block_pointer as *mut u8, header.p_filesz)
                    .copy_from_slice(slice::from_raw_parts(
                        base.byte_add(header.p_offset) as *const u8,
                        header.p_filesz,
                    ));

                // Zero out TLS data beyond `p_filesz`:
                slice::from_raw_parts_mut(
                    tls_block_pointer.byte_add(header.p_filesz) as *mut u8,
                    header.p_memsz - header.p_filesz,
                )
                .fill(0);

                // Push the new block to the pre-allocated vector:
                dynamic_thread_vector.push(DtvItem { tls_block_pointer });
                dynamic_thread_vector.push(DtvItem {
                    generation_counter: 0,
                });

                // Return next TLS block address:
                tls_block_pointer.byte_add(header.p_memsz);
            }

            // Initialize the Thread Control Block (TCB):
            let thread_control_block = tls_allocation_pointer
                .byte_add(tcb_align)
                .byte_add(tls_blocks_size)
                as *mut ThreadControlBlock;

            let thread_pointer_register: *mut () =
                (*thread_control_block).thread_pointee.as_mut_ptr().cast();

            *thread_control_block = ThreadControlBlock {
                thread_pointee: [],
                thread_pointer_register,
                dynamic_thread_vector: dynamic_thread_vector.as_mut_ptr().add(1),
                _padding: [0; 3],
                canary: usize::from_ne_bytes(
                    (*self.pseudorandom_bytes)[..size_of::<usize>()]
                        .try_into()
                        .unwrap(),
                ),
            };

            // Drop `dynamic_thread_vector` without trying to deallocate:
            std::mem::forget(dynamic_thread_vector);

            // Make the thread pointer (which is fs on x86_64) point to the TCB:
            thread_pointer::set_thread_pointer(thread_pointer_register);
        }
    }
}
