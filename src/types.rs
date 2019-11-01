use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::asset_name::AssetName;

static PROJECT_FILENAME: &str = "tarmac-project.toml";
static INPUT_CONFIG_FILENAME: &str = "tarmac.toml";
static MANIFEST_FILENAME: &str = "tarmac-manifest.toml";

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
    pub spritesheet: SpritesheetConfig,
    // TODO: input globs instead of paths?
    // TODO: ignore globs?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpritesheetConfig {
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

impl Default for SpritesheetConfig {
    fn default() -> Self {
        Self {
            max_size: (1024, 1024),
        }
    }
}

/// Configuration that's co-located with the assets it affects.
///
/// This will be set by package and asset authors and collected by a Tarmac
/// project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct InputConfig {
    /// What kind of extra links Tarmac should generate when these assets are
    /// consumed in a project.
    ///
    /// These links can be used by code located near the affected assets to
    /// import them dynamically as if they were normal Lua modules.
    pub codegen: CodegenKind,

    /// Whether the assets affected by this config are allowed to be packed into
    /// spritesheets.
    ///
    /// This isn't enabled by default because special considerations need to be
    /// made in order to correctly handle spritesheets. Not all images are able
    /// to be pre-packed into spritesheets, like images used in `Decal`
    /// instances.
    pub spritesheet_enabled: bool,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            codegen: CodegenKind::None,
            spritesheet_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodegenKind {
    /// Emit no Lua files linking images to their assets.
    ///
    /// This option is useful if another tool is handling the asset mapping, or
    /// assets don't need to be accessed programmatically.
    None,

    /// Emit Lua files that return asset URLs as a string.
    ///
    /// This option is useful for images that will never be packed into a
    /// spritesheet, like `Decal` objects on parts.
    AssetUrl,

    /// Emit Lua files that return a table containing the asset URL, along with
    /// offset and size if the image was packed into a spritesheet.
    ///
    /// The properties in this table are laid out in the same way as the
    /// properties on `ImageLabel` and `ImageButton`:
    ///
    /// * `Image` (string)
    /// * `ImageRectOffset` (Vector2)
    /// * `ImageRectSize` (Vector2)
    UrlAndSlice,
}

/// Tracks the status of all groups, inputs, and outputs as of the last Tarmac
/// sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub groups: HashMap<String, GroupManifest>,
    pub inputs: HashMap<AssetName, InputManifest>,
    pub outputs: HashMap<u64, OutputManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupManifest {
    pub config: GroupConfig,
    pub inputs: HashSet<AssetName>,
    pub outputs: HashSet<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputManifest {
    /// The hierarchical config applied to this config the last time it was part
    /// of an upload.
    pub uploaded_config: InputConfig,

    /// The hexadecimal encoded hash of the contents of this input the last time
    /// it was part of an upload.
    pub uploaded_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputManifest {
    /// The asset ID on Roblox.com that this asset was uploaded to the last time
    /// it was part of an upload.
    pub uploaded_id: u64,
}
