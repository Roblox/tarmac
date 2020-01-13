use crate::{asset_name::AssetName, data::ImageSlice};
use std::collections::HashMap;

use sheep::{Format, SpriteAnchor};

pub struct PackOutput {
    // path: PathBuf,
    dimensions: (u32, u32),
    slices: HashMap<AssetName, ImageSlice>,
}

impl From<&SpriteAnchor> for ImageSlice {
    fn from(anchor: &SpriteAnchor) -> ImageSlice {
        ImageSlice {
            min: (anchor.position.0, anchor.position.1),
            max: (
                anchor.position.0 + anchor.dimensions.0,
                anchor.position.1 + anchor.dimensions.1,
            ),
        }
    }
}

pub struct OutputFormat;

impl Format for OutputFormat {
    type Data = PackOutput;
    // FIXME: Ridiculous cloning
    type Options = Vec<AssetName>;

    fn encode(
        dimensions: (u32, u32),
        sprites: &[SpriteAnchor],
        mut options: Self::Options,
    ) -> Self::Data {
        let slices = sprites
            .iter()
            .map(|anchor| (options[anchor.id].clone(), ImageSlice::from(anchor)))
            .collect();

        PackOutput { dimensions, slices }
    }
}
