use std::{collections::HashMap, fmt};

use crate::{asset_name::AssetName, data::ImageSlice};

pub struct Spritesheet {
    pub dimensions: (u32, u32),
    pub slices: HashMap<AssetName, ImageSlice>,
}

impl Spritesheet {
    pub fn slices(&self) -> impl Iterator<Item = (&AssetName, &ImageSlice)> {
        self.slices.iter()
    }
}

impl fmt::Debug for Spritesheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut lines = String::new();
        self.slices.iter().for_each(|(name, slice)| {
            lines.push_str(format!("\t{}: {:?} {:?}\n", name, slice.min(), slice.max()).as_str());
        });

        write!(
            f,
            "Dimensions: ({}, {})\nInputs:\n{}",
            self.dimensions.0, self.dimensions.1, lines
        )
    }
}
