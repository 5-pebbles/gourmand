use crate::{
    linux::auxiliary_iterator::{AuxiliaryIterator, AT_BASE, AT_ENTRY, AT_PAGE_SIZE},
    utils::no_std_debug_assert,
};

pub(crate) fn relocate_linker(auxiliary_iterator: AuxiliaryIterator) {
    no_std_debug_assert!(auxiliary_iterator
        .clone()
        .any(|value| value.kind == AT_PAGE_SIZE));
    no_std_debug_assert!(auxiliary_iterator
        .clone()
        .any(|value| value.kind == AT_BASE));
    no_std_debug_assert!(auxiliary_iterator
        .clone()
        .any(|value| value.kind == AT_ENTRY));

    // todo!()
}
