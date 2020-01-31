use crate::{geometry::Rect, id::Id};

/// An input to the rectangle packing routines.
///
/// `InputItem` is just a 2D size and a Packos-generated unique identifier. It's
/// expected that consumers will assign meaning to the given IDs and then use
/// them to associate the packing results back to the application's own objects.
#[derive(Debug, Clone, Copy)]
pub struct InputItem {
    pub(crate) id: Id,
    pub(crate) size: (u32, u32),
}

impl InputItem {
    #[inline]
    pub fn new(size: (u32, u32)) -> Self {
        Self {
            id: Id::new(),
            size,
        }
    }

    #[inline]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    #[inline]
    pub fn id(&self) -> Id {
        self.id
    }

    pub(crate) fn area(&self) -> u32 {
        self.size.0 * self.size.1
    }
}

/// An item that was placed by a packing function.
///
/// `OutputItem` corresponds 1:1 to `InputItem` objects that were passed into
/// the packing function. They expose the ID from the input, as well as position
/// and size.
#[derive(Debug, Clone, Copy)]
pub struct OutputItem {
    pub(crate) id: Id,
    pub(crate) rect: Rect,
}

impl OutputItem {
    #[inline]
    pub fn id(&self) -> Id {
        self.id
    }

    #[inline]
    pub fn position(&self) -> (u32, u32) {
        self.rect.pos
    }

    #[inline]
    pub fn size(&self) -> (u32, u32) {
        self.rect.size
    }

    #[inline]
    pub fn min(&self) -> (u32, u32) {
        self.rect.pos
    }

    #[inline]
    pub fn max(&self) -> (u32, u32) {
        self.rect.max()
    }
}

/// The results from running a packing function.
///
/// Currently only exposes the list of buckets that inputs were grouped into. In
/// the future, this struct may also have information about inputs that didn't
/// fit and how efficient the result is.
#[derive(Debug, Clone)]
pub struct PackOutput {
    pub(crate) buckets: Vec<Bucket>,
}

impl PackOutput {
    #[inline]
    pub fn buckets(&self) -> &[Bucket] {
        &self.buckets
    }
}

/// Contains a set of `OutputItem` values that were packed together into the
/// same fixed-size containers.
#[derive(Debug, Clone)]
pub struct Bucket {
    pub(crate) size: (u32, u32),
    pub(crate) items: Vec<OutputItem>,
}

impl Bucket {
    #[inline]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    #[inline]
    pub fn items(&self) -> &[OutputItem] {
        &self.items
    }
}
