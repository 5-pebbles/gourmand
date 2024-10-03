#[derive(Clone, Copy)]
pub(crate) struct ElfHeaderTable<T: Clone + 'static> {
    first: *const T,
    count: u16,
}

impl<T: Clone> ElfHeaderTable<T> {
    pub(crate) fn new(base: usize, offset: usize, count: u16) -> Self {
        Self {
            first: (base + offset) as *const T,
            count,
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&'static T> {
        (index <= self.count as usize).then_some(unsafe { &*self.first.add(index) })
    }

    pub(crate) fn iter(
        &self,
    ) -> core::iter::FromFn<impl FnMut() -> Option<&'static T> + use<'_, T>> {
        self.into_iter()
    }
}

impl<T: Clone + 'static> IntoIterator for ElfHeaderTable<T> {
    type Item = &'static T;
    type IntoIter = core::iter::FromFn<impl FnMut() -> Option<&'static T>>;

    fn into_iter(self) -> Self::IntoIter {
        // its not perfect but it works ;)
        let mut index = 0;
        core::iter::from_fn(move || {
            self.get(index).map(|h| {
                index += 1;
                h
            })
        })
    }
}

impl<T: Clone + 'static> IntoIterator for &ElfHeaderTable<T> {
    type Item = &'static T;
    type IntoIter = core::iter::FromFn<impl FnMut() -> Option<&'static T>>;

    fn into_iter(self) -> Self::IntoIter {
        let mut index = 0;
        core::iter::from_fn(move || {
            self.get(index).map(|h| {
                index += 1;
                h
            })
        })
    }
}
