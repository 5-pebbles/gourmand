use core::{ptr::null_mut, slice, str};

#[derive(Clone, Copy)]
pub struct EnvironmentIterator(*mut *mut u8);

impl EnvironmentIterator {
    pub(crate) fn new(environment_pointer: *mut *mut u8) -> Self {
        Self(environment_pointer)
    }

    pub(crate) fn into_inner(self) -> *mut *mut u8 {
        self.0
    }
}

impl Iterator for EnvironmentIterator {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let this = *self.0;
            if this.is_null() {
                return None;
            }

            let len = (0..).take_while(|&i| *this.add(i) != 0).count();
            let slice = slice::from_raw_parts(this, len);
            // The check segfaults in this context. :/
            // This is the same as just calling mem::transmute.
            let s = str::from_utf8_unchecked(slice);

            self.0 = self.0.add(1);

            Some(s)
        }
    }
}
