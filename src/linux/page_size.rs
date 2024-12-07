use std::sync::OnceLock;

pub static PAGE_SIZE: OnceLock<usize> = OnceLock::new();

pub(crate) fn set_page_size(page_size: usize) {
    let _ = PAGE_SIZE.set(page_size);
}

pub(crate) fn get_page_size() -> usize {
    *PAGE_SIZE.get().expect("Page size not initialized")
}

pub(crate) fn get_page_start(address: usize) -> usize {
    address & !(get_page_size() - 1)
}

pub(crate) fn get_page_offset(address: usize) -> usize {
    address & (get_page_size() - 1)
}

pub(crate) fn get_page_end(address: usize) -> usize {
    get_page_start(address + get_page_size() - 1)
}
