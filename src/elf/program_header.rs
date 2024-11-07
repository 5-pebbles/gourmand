pub(crate) const PT_LOAD: u32 = 1;
pub(crate) const PT_DYNAMIC: u32 = 2;
pub(crate) const PT_PHDR: u32 = 6;
pub(crate) const PT_TLS: u32 = 7;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ProgramHeader {
    pub p_type: u32,
    #[cfg(target_pointer_width = "64")]
    pub p_flags: u32,
    pub p_offset: usize,
    pub p_vaddr: usize,
    pub p_paddr: usize,
    pub p_filesz: usize,
    pub p_memsz: usize,
    #[cfg(target_pointer_width = "32")]
    pub p_flags: u32,
    pub p_align: usize,
}
