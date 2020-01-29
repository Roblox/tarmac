// TODO: Should this Id type be specified by the user of the library? Can we
// generate them internally and give them to the consumer as they construct
// `InputRect` objects?
pub type Id = usize;

#[derive(Debug, Clone, Copy)]
pub struct InputRect {
    id: Id,
    size: (u32, u32),
}

impl InputRect {
    pub fn new(size: (u32, u32)) -> Self {
        Self { id: 0, size }
    }

    fn area(&self) -> u32 {
        self.size.0 * self.size.1
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OutputRect {
    id: Id,
    size: (u32, u32),
    pos: (u32, u32),
}

pub struct PackResult {
    buckets: Vec<PackBucket>,
}

pub struct PackBucket {
    size: (u32, u32),
    items: Vec<OutputRect>,
}

pub struct SimplePacker {
    min_size: (u32, u32),
    max_size: (u32, u32),
}

impl SimplePacker {
    pub fn new() -> Self {
        Self {
            min_size: (128, 128),
            max_size: (1024, 1024),
        }
    }

    pub fn pack<I: IntoIterator<Item = InputRect>>(&self, items: I) -> PackResult {
        let mut items: Vec<_> = items.into_iter().collect();
        items.sort_by_key(InputRect::area);

        let mut remaining_items = items.as_slice();
        let mut buckets = Vec::new();

        while !remaining_items.is_empty() {
            // TODO: Compute minimum size from total area of input images, rounded
            // up to nearest po2 and clamped to max_size.
            let mut current_size = self.min_size;

            loop {
                let (bucket, next_remaining) = Self::pack_one_bucket(remaining_items, current_size);

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

        PackResult { buckets }
    }

    fn pack_one_bucket(sorted_items: &[InputRect], size: (u32, u32)) -> (PackBucket, &[InputRect]) {
        unimplemented!()
    }
}
