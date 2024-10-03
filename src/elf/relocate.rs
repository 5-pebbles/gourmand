#[cfg(target_arch = "x86_64")]
pub(crate) const R_RELATIVE: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct ElfRela {
    pub r_offset: usize,
    pub r_sym: u32,
    pub r_type: u32,
    pub r_addend: usize,
}
