use core::ffi::c_void;

use super::environment_variables::EnvironmentIter;

pub(crate) const AT_NULL: usize = 0;
pub(crate) const AT_PHDR: usize = 3;
pub(crate) const AT_PHENT: usize = 4;
pub(crate) const AT_PHNUM: usize = 5;
pub(crate) const AT_PAGE_SIZE: usize = 6;
pub(crate) const AT_BASE: usize = 7;
pub(crate) const AT_ENTRY: usize = 9;
pub(crate) const AT_RANDOM: usize = 25;

/// A union resolved by the a_type field of the parent auxiliary vector item.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) union AuxiliaryVectorUnion {
    pub a_val: usize,
    pub a_ptr: *mut c_void,
}

/// An item in the auxiliary vector.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct AuxiliaryVectorItem {
    pub a_type: usize,
    // NOTE: I couldn't find good documentation on this field; glibc's `getauxval` returns a usize, but I think it really represents union.
    pub a_un: AuxiliaryVectorUnion,
}

/// An iterator over a `AT_NULL` terminated list of auxiliary vector items.
///
/// The inital pointer can be found two null-bytes after the end of the environmant pointers:
///
/// ```no_run
/// |---------------------|
/// | arg_count           |
/// |---------------------|
/// | arg_values...       |
/// |---------------------|
/// | null                |
/// |---------------------|
/// | env_pointers...     |
/// |---------------------|
/// | null                |
/// |---------------------|
/// | null                |
/// |---------------------|
/// | auxiliary_vector... |
/// |---------------------|
/// | null                |
/// |---------------------|
/// | ...                 |
/// |---------------------|
/// ```
#[derive(Clone, Copy)]
pub(crate) struct AuxiliaryVectorIter(*const AuxiliaryVectorItem);

impl AuxiliaryVectorIter {
    /// Initializes a new `AuxiliaryVectorIter` from a 16-byte aligned and pre-offset `*const AuxiliaryVectorItem` pointer.
    pub(crate) fn new(auxiliary_vector_pointer: *const AuxiliaryVectorItem) -> Self {
        Self(auxiliary_vector_pointer)
    }

    /// Calculates and initializes a new `AuxiliaryVectorIter` from an `EnvironmentIter`.
    pub(crate) fn from_environment_iter(environment_iterator: EnvironmentIter) -> Self {
        let mut environment_pointer = environment_iterator.into_inner();

        unsafe {
            while !(*environment_pointer).is_null() {
                environment_pointer = environment_pointer.add(1);
            }

            Self::new(environment_pointer.add(1) as *const AuxiliaryVectorItem)
        }
    }

    /// Extracts the inner pointer to the next item consuming the `AuxiliaryVectorIter`.
    pub(crate) fn into_inner(self) -> *const AuxiliaryVectorItem {
        self.0
    }
}

impl Iterator for AuxiliaryVectorIter {
    type Item = AuxiliaryVectorItem;

    fn next(&mut self) -> Option<Self::Item> {
        let item = unsafe { *self.0 };

        // If we are at the end of the list, return `None` and don't progress.
        if item.a_type == AT_NULL {
            return None;
        }

        // Advance to the next item
        self.0 = unsafe { self.0.add(1) };

        Some(item)
    }
}
