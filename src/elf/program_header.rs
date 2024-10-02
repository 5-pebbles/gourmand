use crate::elf::header::ElfHeader;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub struct ElfProgramHeader {
    pub kind: u32,
    #[cfg(target_pointer_width = "64")]
    pub segment_dependent_flags: u32,
    pub segment_offset: usize,
    pub segment_virtual_address: usize,
    pub segment_physical_address: usize,
    pub segment_size_in_image: usize,
    pub segment_size_in_memory: usize,
    #[cfg(target_pointer_width = "32")]
    pub segment_dependent_flags: u32,
    pub segment_alignment_constraint: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct ElfProgramHeaderTable {
    start: *const ElfProgramHeader,
    len: u16,
}

impl ElfProgramHeaderTable {
    pub(crate) fn new(base: usize, elf_header: ElfHeader) -> Self {
        Self {
            start: (base + elf_header.program_header_table_offset) as *const ElfProgramHeader,
            len: elf_header.program_header_table_entry_count,
        }
    }

    pub(crate) fn get<'a>(self, index: usize) -> Option<&'a ElfProgramHeader> {
        (index <= self.len as usize).then_some(unsafe { &*self.start.add(index) })
    }
}

impl IntoIterator for ElfProgramHeaderTable {
    type Item = ElfProgramHeader;
    type IntoIter = core::iter::FromFn<impl FnMut() -> Option<ElfProgramHeader>>;

    fn into_iter(self) -> Self::IntoIter {
        // its not perfect but it works ;)
        let mut index = 0;
        core::iter::from_fn(move || {
            self.get(index).map(|h| {
                index += 1;
                *h
            })
        })
    }
}
