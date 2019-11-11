use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

static PROJECT_CONFIG_FILENAME: &str = "tarmac-project.toml";

/// Project-level configuration. Defined once, where Tarmac is run from, in a
/// `tarmac-project.toml` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub groups: HashMap<String, GroupConfig>,
}

impl ProjectConfig {
    pub fn read_from_folder<P: AsRef<Path>>(
        folder_path: P,
    ) -> Result<Option<Self>, ProjectConfigError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(PROJECT_CONFIG_FILENAME);

        let contents = match fs::read(file_path) {
            Ok(contents) => contents,
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(None);
            }
            other => other.context(Io { file_path })?,
        };

        let config = toml::from_slice(&contents).context(Toml { file_path })?;

        Ok(Some(config))
    }
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

#[derive(Debug, Snafu)]
pub enum ProjectConfigError {
    Toml {
        file_path: PathBuf,
        source: toml::de::Error,
    },

    Io {
        file_path: PathBuf,
        source: io::Error,
    },
}
