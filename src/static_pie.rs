use std::{
    cmp::max,
    marker::PhantomData,
    ptr::{null, null_mut},
    slice,
};

use crate::{
    arch::{
        exit::exit,
        mmap::{mmap, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE},
        relocation::relocate,
        thread_pointer::set_thread_pointer,
    },
    elf::{
        dynamic_array::{DynamicArrayItem, DynamicArrayIter, DT_RELA, DT_RELAENT, DT_RELASZ},
        header::{ElfHeader, ET_DYN},
        program_header::{ProgramHeader, PT_DYNAMIC, PT_PHDR, PT_TLS},
        relocate::{Rela, Relocatable, RelocationSlices},
        symbol::Symbol,
        thread_local_storage::ThreadControlBlock,
    },
    syscall_debug_assert,
};

fn round_up_to_boundary(address: usize, boundary: usize) -> usize {
    (address + (boundary - 1)) & boundary.wrapping_neg()
}

pub struct Ingredients;
pub struct Baked;

/// A struct representing a statically relocatable Position Independent Executable (PIE). 🥧
///
/// # Operations
///
/// The struct supports the following operations:
/// - Performing runtime relocations
/// - Initializing thread-local storage
pub struct StaticPie<T> {
    base_address: *const (),
    relocation_slices: RelocationSlices,
    tls_program_header: Option<&'static ProgramHeader>,
    pseudorandom_bytes: *const [u8; 16],
    phantom_data: PhantomData<T>,
}

impl Relocatable for StaticPie<Ingredients> {
    fn base(&self) -> *const () {
        self.base_address
    }

    fn symbol(&self, _symbol_index: usize) -> Symbol {
        syscall_debug_assert!(false);
        exit(3);
    }

    fn relocation_slices(&self) -> RelocationSlices {
        self.relocation_slices
    }
}

impl StaticPie<Ingredients> {
    pub unsafe fn from_base(
        base: *const (),
        pseudorandom_bytes: *const [u8; 16],
    ) -> StaticPie<Ingredients> {
        // ELf Header:
        let header = &*(base as *const ElfHeader);
        syscall_debug_assert!(header.e_type == ET_DYN);
        syscall_debug_assert!(header.e_phentsize == size_of::<ProgramHeader>() as u16);

        // Program Headers:
        let program_header_table = slice::from_raw_parts(
            base.byte_add(header.e_phoff) as *const ProgramHeader,
            header.e_phnum as usize,
        );

        let (mut dynamic_program_header, mut tls_program_header) = (None, None);
        for header in program_header_table {
            match header.p_type {
                PT_DYNAMIC => dynamic_program_header = Some(header),
                PT_TLS => tls_program_header = Some(header),
                _ => (),
            }
        }
        syscall_debug_assert!(dynamic_program_header.is_some());

        Self::build(
            base,
            dynamic_program_header.unwrap_unchecked(),
            tls_program_header,
            pseudorandom_bytes,
        )
    }

    pub unsafe fn from_program_headers(
        program_header_table: &'static [ProgramHeader],
        pseudorandom_bytes: *const [u8; 16],
    ) -> StaticPie<Ingredients> {
        let (mut base, mut dynamic_program_header, mut tls_program_header) = (null(), None, None);
        for header in program_header_table {
            match header.p_type {
                PT_PHDR => {
                    base = program_header_table.as_ptr().byte_sub(header.p_vaddr) as *const ();
                }
                PT_DYNAMIC => dynamic_program_header = Some(header),
                PT_TLS => tls_program_header = Some(header),
                _ => (),
            }
        }
        syscall_debug_assert!(dynamic_program_header.is_some());

        Self::build(
            base,
            dynamic_program_header.unwrap_unchecked(),
            tls_program_header,
            pseudorandom_bytes,
        )
    }

    #[must_use]
    unsafe fn build(
        base: *const (),
        dynamic_program_header: &ProgramHeader,
        tls_program_header: Option<&'static ProgramHeader>,
        pseudorandom_bytes: *const [u8; 16],
    ) -> StaticPie<Ingredients> {
        // Dynamic Arrary:
        let dynamic_array = DynamicArrayIter::new(
            base.byte_add(dynamic_program_header.p_vaddr) as *const DynamicArrayItem
        );
        syscall_debug_assert!(dynamic_array.clone().count() != 0);

        let mut rela_pointer: *const Rela = null();
        let mut rela_count = 0;

        for item in dynamic_array {
            match item.d_tag {
                DT_RELA => {
                    rela_pointer = base.byte_add(item.d_un.d_ptr.addr()) as *const Rela;
                }
                DT_RELASZ => {
                    rela_count = item.d_un.d_val / core::mem::size_of::<Rela>();
                }
                #[cfg(debug_assertions)]
                DT_RELAENT => {
                    syscall_debug_assert!(item.d_un.d_val as usize == size_of::<Rela>())
                }
                _ => (),
            }
        }

        syscall_debug_assert!(rela_pointer != null());
        let rela_slice = slice::from_raw_parts(rela_pointer, rela_count);

        StaticPie::<Ingredients> {
            base_address: base,
            relocation_slices: RelocationSlices { rela_slice },
            tls_program_header,
            pseudorandom_bytes,
            phantom_data: PhantomData,
        }
    }
}

impl StaticPie<Ingredients> {
    #[must_use]
    #[inline(always)]
    pub fn relocate_to_oven(self) -> StaticPie<Baked> {
        unsafe { relocate(&self) };

        StaticPie::<Baked> {
            phantom_data: PhantomData::<Baked>,
            ..self
        }
    }
}

impl StaticPie<Baked> {
    #[inline(always)]
    pub unsafe fn allocate_tls_in_stomach(self) {
        // Static Thread Local Storage [before Thread Pointer]:
        //      ┌----------------------------┐
        //      |    TCB & TLS Alignment     |     ┌---------------------┐
        //      |----------------------------|  <- |    tls-offset[1]    |
        //      |      Static TLS Block      |     |---------------------|
        //      |----------------------------|  <- | Thread Pointer (TP) |
        // ┌--- | Thread Control Block (TCB) |     └---------------------┘
        // |    └----------------------------┘
        // |
        // |   ┌------------------┐
        // └-> | Null Dtv Pointer |
        //     └------------------┘
        let Some(tls_program_header) = self.tls_program_header else {
            return;
        };

        let tcb_and_tls_align = max(tls_program_header.p_align, align_of::<ThreadControlBlock>());
        let tls_blocks_size_and_align =
            round_up_to_boundary(tls_program_header.p_memsz, tls_program_header.p_align);
        let tcb_size = size_of::<ThreadControlBlock>();

        let required_size = tcb_and_tls_align + tls_blocks_size_and_align + tcb_size;
        let tls_allocation_pointer = mmap(
            null_mut(),
            required_size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1, // file descriptor (-1 for anonymous mapping)
            0,  // offset
        );
        syscall_debug_assert!(tls_allocation_pointer.addr() % tcb_and_tls_align == 0);

        let tls_block_pointer = tls_allocation_pointer.byte_add(tcb_and_tls_align);

        // Initialize the TLS data from template image:
        slice::from_raw_parts_mut(tls_block_pointer, tls_program_header.p_filesz).copy_from_slice(
            slice::from_raw_parts(
                self.base_address.byte_add(tls_program_header.p_offset) as *const u8,
                tls_program_header.p_filesz,
            ),
        );

        // Zero out TLS data beyond `p_filesz`:
        slice::from_raw_parts_mut(
            tls_block_pointer.byte_add(tls_program_header.p_filesz),
            tls_program_header.p_memsz - tls_program_header.p_filesz,
        )
        .fill(0);

        // Initialize the Thread Control Block (TCB):
        let thread_control_block = tls_allocation_pointer
            .byte_add(tcb_and_tls_align)
            .byte_add(tls_blocks_size_and_align)
            as *mut ThreadControlBlock;

        let thread_pointer_register: *mut () =
            (*thread_control_block).thread_pointee.as_mut_ptr().cast();

        *thread_control_block = ThreadControlBlock {
            thread_pointee: [],
            thread_pointer_register,
            dynamic_thread_vector: null_mut(),
            _padding: [0; 3],
            canary: usize::from_ne_bytes(
                (*self.pseudorandom_bytes)[..size_of::<usize>()]
                    .try_into()
                    .unwrap(),
            ),
        };

        // Make the thread pointer (which is fs on x86_64) point to the TCB:
        set_thread_pointer(thread_pointer_register);
    }
}
