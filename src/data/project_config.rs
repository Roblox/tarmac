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
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    #[serde(default)]
    pub groups: HashMap<String, GroupConfig>,

    #[serde(skip)]
    pub file_path: PathBuf,
}

impl ProjectConfig {
    pub fn read_from_folder_or_file<P: AsRef<Path>>(path: P) -> Result<Self, ProjectConfigError> {
        let path = path.as_ref();
        let meta = fs::metadata(path).context(Io { path })?;

        if meta.is_file() {
            Self::read_from_file(path)
        } else {
            Self::read_from_folder(path)
        }
    }

    fn read_from_file(path: &Path) -> Result<Self, ProjectConfigError> {
        let contents = fs::read(path).context(Io { path })?;

        let mut config: Self = toml::from_slice(&contents).context(Toml { path })?;
        config.file_path = path.to_owned();

        Ok(config)
    }

    fn read_from_folder<P: AsRef<Path>>(folder_path: P) -> Result<Self, ProjectConfigError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(PROJECT_CONFIG_FILENAME);

        Self::read_from_file(file_path)
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
    #[snafu(display("{} in {}", source, path.display()))]
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[snafu(display("{} in {}", source, path.display()))]
    Io { path: PathBuf, source: io::Error },
}
