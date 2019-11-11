use std::collections::HashMap;

use serde::{Deserialize, Serialize};

static PROJECT_CONFIG_FILENAME: &str = "tarmac-project.toml";

/// Project-level configuration. Defined once, where Tarmac is run from, in a
/// `tarmac-project.toml` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub groups: HashMap<String, GroupConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    /// All of the paths that Tarmac should search to populate this group with
    /// inputs.
    pub paths: Vec<String>,

    /// Defines the spritesheet strategy to use for packing assets dynamically
    /// within this group.
    ///
    /// Not all assets can be packed into spritesheets, which is controlled by
    /// configuration co-located with assets.
    pub spritesheet: GroupSpritesheetConfig,
    // TODO: input globs instead of paths?
    // TODO: ignore globs?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GroupSpritesheetConfig {
    /// Whether to attempt to collect images into spritesheets.
    pub enabled: bool,

    /// The maximum dimensions of generated spritesheets.
    ///
    /// If Tarmac runs out of room in a spritesheet, images will be put into
    /// multiple spritesheet images.
    pub max_size: (usize, usize),
    // TODO: packing algorithm?
    // TODO: preferred image format?
}

impl Default for GroupSpritesheetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_size: (1024, 1024),
        }
    }
}
