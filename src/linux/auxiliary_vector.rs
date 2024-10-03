use super::environment_iterator::EnvironmentIterator;

pub(crate) const AT_NULL: usize = 0;
pub(crate) const AT_PAGE_SIZE: usize = 6;
pub(crate) const AT_BASE: usize = 7;
pub(crate) const AT_ENTRY: usize = 9;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct AuxiliaryVectorItem {
    pub a_type: usize,
    pub a_val: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct AuxiliaryVectorIter(*const AuxiliaryVectorItem);

impl AuxiliaryVectorIter {
    pub(crate) fn new(auxiliary_vector_pointer: *const AuxiliaryVectorItem) -> Self {
        Self(auxiliary_vector_pointer)
    }

    pub(crate) fn into_inner(self) -> *const AuxiliaryVectorItem {
        self.0
    }

    pub(crate) fn from_environment_iterator(environment_iterator: EnvironmentIterator) -> Self {
        let mut environment_pointer = environment_iterator.into_inner();

        unsafe {
            while !(*environment_pointer).is_null() {
                environment_pointer = environment_pointer.add(1);
            }

            Self::new(environment_pointer.add(1) as *const AuxiliaryVectorItem)
        }
    }
}

impl Iterator for AuxiliaryVectorIter {
    type Item = AuxiliaryVectorItem;

    fn next(&mut self) -> Option<Self::Item> {
        let this = unsafe { *self.0 };
        if this.a_type == AT_NULL {
            return None;
        }
        self.0 = unsafe { self.0.add(1) };
        Some(this)
    }
}
