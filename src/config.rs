use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

static CONFIG_FILENAME: &str = "tarmac.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default: ConfigEntry,
}

impl Config {
    pub fn read_from_folder<P: AsRef<Path>>(folder_path: P) -> Result<Option<Self>, ConfigError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(CONFIG_FILENAME);

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
#[serde(default, rename_all = "kebab-case")]
pub struct ConfigEntry {
    pub codegen: CodegenKind,
    pub can_spritesheet: bool,
}

impl Default for ConfigEntry {
    fn default() -> Self {
        Self {
            codegen: CodegenKind::None,
            can_spritesheet: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodegenKind {
    None,
    AssetUrl,
    Slice,
}

#[derive(Debug, Snafu)]
pub enum ConfigError {
    Toml {
        file_path: PathBuf,
        source: toml::de::Error,
    },

    Io {
        file_path: PathBuf,
        source: io::Error,
    },
}
