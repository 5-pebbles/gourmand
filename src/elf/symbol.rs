#[repr(C)]
pub(crate) struct Symbol {
    pub st_name: u32,
    #[cfg(target_pointer_width = "32")]
    pub st_value: usize,
    #[cfg(target_pointer_width = "32")]
    pub st_size: usize,
    pub st_info: u8,
    pub st_other: u8,
    pub st_shndx: u16,
    #[cfg(target_pointer_width = "64")]
    pub st_value: usize,
    #[cfg(target_pointer_width = "64")]
    pub st_size: usize,
}
