use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

static INPUT_CONFIG_FILENAME: &str = "tarmac.toml";

/// Configuration that's co-located with the assets it affects.
///
/// This will be set by package and asset authors and collected by a Tarmac
/// project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
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

impl InputConfig {
    pub fn read_from_folder<P: AsRef<Path>>(folder_path: P) -> Result<Self, InputConfigError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(INPUT_CONFIG_FILENAME);

        let contents = fs::read(file_path).context(Io { path: file_path })?;
        let config = toml::from_slice(&contents).context(Toml { path: file_path })?;

        Ok(config)
    }
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

#[derive(Debug, Snafu)]
pub enum InputConfigError {
    Io {
        path: PathBuf,
        source: io::Error,
    },

    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl InputConfigError {
    pub fn is_not_found(&self) -> bool {
        match self {
            InputConfigError::Io { source, .. } => source.kind() == io::ErrorKind::NotFound,
            _ => false,
        }
    }
}
