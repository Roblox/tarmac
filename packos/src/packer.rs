use std::cmp::Reverse;

use crate::{geometry::Rect, id::Id};

#[derive(Debug, Clone, Copy)]
pub struct InputItem {
    id: Id,
    size: (u32, u32),
}

impl InputItem {
    pub fn new(size: (u32, u32)) -> Self {
        Self {
            id: Id::new(),
            size,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    fn area(&self) -> u32 {
        self.size.0 * self.size.1
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OutputItem {
    id: Id,
    rect: Rect,
}

impl OutputItem {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn position(&self) -> (u32, u32) {
        self.rect.pos
    }

    pub fn size(&self) -> (u32, u32) {
        self.rect.size
    }

    pub fn max(&self) -> (u32, u32) {
        self.rect.max()
    }
}

#[derive(Debug, Clone)]
pub struct PackOutput {
    buckets: Vec<Bucket>,
}

impl PackOutput {
    pub fn buckets(&self) -> &[Bucket] {
        &self.buckets
    }
}

#[derive(Debug, Clone)]
pub struct Bucket {
    size: (u32, u32),
    items: Vec<OutputItem>,
}

impl Bucket {
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn items(&self) -> &[OutputItem] {
        &self.items
    }
}

pub struct SimplePacker {
    min_size: (u32, u32),
    max_size: (u32, u32),
    padding: u32,
}

impl SimplePacker {
    pub fn new() -> Self {
        Self {
            min_size: (128, 128),
            max_size: (1024, 1024),
            padding: 0,
        }
    }

    pub fn max_size(self, max_size: (u32, u32)) -> Self {
        Self { max_size, ..self }
    }

    pub fn padding(self, padding: u32) -> Self {
        Self { padding, ..self }
    }

    pub fn pack<I: IntoIterator<Item = InputItem>>(&self, items: I) -> PackOutput {
        let mut remaining_items: Vec<_> = items.into_iter().collect();
        remaining_items.sort_by_key(|input| Reverse(input.area()));

        for item in &mut remaining_items {
            item.size = (item.size.0 + self.padding, item.size.1 + self.padding);
        }

        let num_items = remaining_items.len();
        log::trace!("Packing {} items", num_items);

        let mut buckets = Vec::new();

        while !remaining_items.is_empty() {
            // TODO: Compute minimum size from total area of remaining images,
            // rounded up to nearest po2 and clamped to max_size.
            let mut current_size = self.min_size;

            loop {
                let (bucket, next_remaining) =
                    Self::pack_one_bucket(&remaining_items, current_size);

                // If this size was large enough to contain the rest of the
                // images, we're done packing!
                if next_remaining.is_empty() {
                    buckets.push(bucket);
                    remaining_items = next_remaining;
                    break;
                }

                // Otherwise, we can try to re-pack this set of images into a
                // larger bucket to try to minimize the total number of buckets
                // we use.
                if current_size.0 < self.max_size.0 || current_size.1 < self.max_size.1 {
                    current_size = (
                        (current_size.0 * 2).min(self.max_size.0),
                        (current_size.1 * 2).min(self.max_size.1),
                    );
                } else {
                    // We're already at the max bucket size, so this is the
                    // smallest number of buckets we'll get.
                    buckets.push(bucket);
                    remaining_items = next_remaining;
                    break;
                }
            }
        }

        for bucket in &mut buckets {
            for item in &mut bucket.items {
                item.rect.size = (
                    item.rect.size.0 - self.padding,
                    item.rect.size.1 - self.padding,
                );
            }
        }

        log::trace!(
            "Finished packing {} items into {} buckets",
            num_items,
            buckets.len()
        );

        PackOutput { buckets }
    }

    fn pack_one_bucket(
        remaining_items: &[InputItem],
        bucket_size: (u32, u32),
    ) -> (Bucket, Vec<InputItem>) {
        log::trace!(
            "Trying to pack {} remaining items into bucket of size {:?}",
            remaining_items.len(),
            bucket_size
        );

        let mut anchors = vec![(0, 0)];
        let mut items: Vec<OutputItem> = Vec::new();
        let mut unpacked_items = Vec::new();

        for input_item in remaining_items {
            log::trace!(
                "For item {:?} ({}x{}), evaluating these anchors: {:?}",
                input_item.id,
                input_item.size.0,
                input_item.size.1,
                anchors
            );

            let fit_anchor = anchors.iter().copied().position(|anchor| {
                let potential_rect = Rect {
                    pos: anchor,
                    size: input_item.size,
                };

                let fits_with_others = items
                    .iter()
                    .all(|packed_item| !potential_rect.intersects(&packed_item.rect));

                let max = potential_rect.max();
                let fits_in_bucket = max.0 < bucket_size.0 && max.1 < bucket_size.1;

                fits_with_others && fits_in_bucket
            });

            if let Some(index) = fit_anchor {
                let anchor = anchors.remove(index);

                log::trace!("Fit at anchor {:?}", anchor);

                let new_anchor_hor = (anchor.0 + input_item.size.0, anchor.1);
                if new_anchor_hor.0 < bucket_size.0 && new_anchor_hor.1 < bucket_size.1 {
                    anchors.push(new_anchor_hor);
                }

                let new_anchor_ver = (anchor.0, anchor.1 + input_item.size.1);
                if new_anchor_ver.0 < bucket_size.0 && new_anchor_ver.1 < bucket_size.1 {
                    anchors.push(new_anchor_ver);
                }

                let output_item = OutputItem {
                    id: input_item.id,
                    rect: Rect {
                        pos: anchor,
                        size: input_item.size,
                    },
                };
                items.push(output_item);
            } else {
                log::trace!("Did not fit in this bucket.");

                unpacked_items.push(*input_item);
            }
        }

        let bucket = Bucket {
            size: bucket_size,
            items,
        };

        (bucket, unpacked_items)
    }
}
