use std::{collections::HashMap, fmt};

use sheep::{Format, SpriteAnchor};

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

impl From<&SpriteAnchor> for ImageSlice {
    fn from(anchor: &SpriteAnchor) -> ImageSlice {
        ImageSlice::new(
            (anchor.position.0, anchor.position.1),
            (
                anchor.position.0 + anchor.dimensions.0,
                anchor.position.1 + anchor.dimensions.1,
            ),
        )
    }
}

pub struct OutputFormat;

impl Format for OutputFormat {
    type Data = Spritesheet;
    // FIXME: Quite a bit of cloning here, might end up wanting to box this I
    // guess?
    type Options = Vec<AssetName>;

    fn encode(
        dimensions: (u32, u32),
        sprites: &[SpriteAnchor],
        options: Self::Options,
    ) -> Self::Data {
        let slices = sprites
            .iter()
            .map(|anchor| (options[anchor.id].clone(), ImageSlice::from(anchor)))
            .collect();

        Spritesheet { dimensions, slices }
    }
}
