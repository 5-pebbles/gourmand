// I need an allocator that doesn't depend on any external libraries.
// This is because we need to allocate before performing runtime linking and relocations.

use core::{
    alloc::{GlobalAlloc, Layout},
    cmp::max,
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{arch, linux::io_macros::syscall_debug_assert};

#[global_allocator]
pub(crate) static mut ALLOCATOR: Allocator = Allocator::new();

const MAX_SUPPORTED_ALIGN: usize = 4096;

pub(crate) struct Allocator {
    // I can't use OnceCell/OnceLock because they aren't sync
    page_size: AtomicUsize,
    thread_cache: ThreadCache,
}

impl Allocator {
    pub const fn new() -> Self {
        Allocator {
            page_size: AtomicUsize::new(0),
            thread_cache: ThreadCache::new(),
        }
    }

    pub fn initialize(&mut self, page_size: usize) {
        syscall_debug_assert!(self.page_size.load(Ordering::Relaxed) == 0);
        self.page_size.store(page_size, Ordering::Release);
    }

    fn align_layout_to_page_size(&self, layout: Layout) -> Layout {
        let page_size = self.page_size.load(Ordering::Acquire);
        let aligned_layout = layout.align_to(max(layout.align(), page_size));
        syscall_debug_assert!(aligned_layout.is_ok());
        aligned_layout.unwrap()
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > MAX_SUPPORTED_ALIGN {
            return null_mut();
        }
        let size = layout.pad_to_align().size();
        match size {
            _ => arch::mmap(self.align_layout_to_page_size(layout).pad_to_align().size()),
        }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        arch::munmap(
            pointer,
            self.align_layout_to_page_size(layout).pad_to_align().size(),
        );
    }
}

struct ThreadCache {}

impl ThreadCache {
    pub const fn new() -> Self {
        Self {}
    }
}
