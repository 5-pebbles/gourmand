/// An ELF relocation entry with an addend.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Rela {
    pub r_offset: usize,
    pub r_info: usize,
    pub r_addend: isize,
}

impl Rela {
    /// Extracts the symbol table index from the `r_info` field.
    pub(crate) fn r_sym(&self) -> u32 {
        #[cfg(target_pointer_width = "64")]
        {
            (self.r_info >> 32) as u32
        }
        #[cfg(target_pointer_width = "32")]
        {
            (self.r_info >> 8) as u32
        }
    }

    /// Extracts the relocation type from the `r_info` field.
    pub(crate) fn r_type(&self) -> u32 {
        #[cfg(target_pointer_width = "64")]
        {
            (self.r_info & 0xFFFFFFFF) as u32
        }
        #[cfg(target_pointer_width = "32")]
        {
            (self.r_info & 0xFF) as u32
        }
    }
}
