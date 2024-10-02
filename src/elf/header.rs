#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ElfHeader {
    pub elf_identification: [u8; 16],
    pub kind: u16,
    pub instruction_set_architecture: u16,
    pub elf_version: u32,
    pub entry_point: usize,
    pub program_header_table_offset: usize,
    pub section_header_table_offset: usize,
    pub processor_specific_flags: u32,
    pub elf_header_size: u16,
    pub program_header_table_entry_size: u16,
    pub program_header_table_entry_count: u16,
    pub section_header_table_entry_size: u16,
    pub section_header_table_entry_count: u16,
    pub section_header_table_entry_index_for_section_names: u16,
}
