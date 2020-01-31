use std::{
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

static LAST_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(NonZeroUsize);

impl Id {
    pub(crate) fn new() -> Self {
        let id = LAST_ID.fetch_add(1, Ordering::SeqCst);
        Id(NonZeroUsize::new(id).unwrap())
    }
}
