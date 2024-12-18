#[repr(C)]
pub struct ThreadControlBlock {
    pub thread_pointee: [(); 0],
    pub thread_pointer_register: *mut (),
    pub dynamic_thread_vector: *mut DtvItem,
    pub _padding: [usize; 3],
    pub canary: usize,
}

#[repr(C)]
pub union DtvItem {
    pub length: usize,
    pub generation_counter: usize,
    pub tls_block_pointer: *mut (),
}
