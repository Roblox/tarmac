use std::{
    io,
    path::{Path, PathBuf},
};

use fs_err as fs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::glob::Glob;

static CONFIG_FILENAME: &str = "tarmac.toml";

/// Configuration for Tarmac, contained in a tarmac.toml file.
///
/// Tarmac is started from a top-level tarmac.toml file. Config files can
/// include other config files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// The name of the project, currently only used in debugging.
    pub name: String,

    /// The maximum size that any packed spritesheets should be. Only applies if
    /// this config is the root config file.
    #[serde(default = "default_max_spritesheet_size")]
    pub max_spritesheet_size: (u32, u32),

    /// A path to a folder where any assets contained in the project should be
    /// stored. Each asset's name will match its asset ID.
    pub asset_cache_path: Option<PathBuf>,

    /// A path to a file where Tarmac will write a list of all of the asset URLs
    /// referred to by this project.
    pub asset_list_path: Option<PathBuf>,

    /// If specified, requires that all uploaded assets are uploaded to the
    /// given group. Attempting to sync will fail if the authenticated user does
    /// not have access to create assets on the group.
    pub upload_to_group_id: Option<u64>,

    /// A list of paths that Tarmac should search in to find other Tarmac
    /// projects.
    ///
    /// Any found projects will have their inputs merged into this project.
    #[serde(default)]
    pub includes: Vec<PathBuf>,

    /// A list of input glob paths and options that Tarmac should use to
    /// discover assets that it should manage.
    #[serde(default)]
    pub inputs: Vec<InputConfig>,

    /// The path that this config came from. Paths from this config should be
    /// relative to the folder containing this file.
    #[serde(skip)]
    pub file_path: PathBuf,
}

impl Config {
    pub fn read_from_folder_or_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let meta = fs::metadata(path)?;

        if meta.is_file() {
            Self::read_from_file(path)
        } else {
            Self::read_from_folder(path)
        }
    }

    pub fn read_from_folder<P: AsRef<Path>>(folder_path: P) -> Result<Self, ConfigError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(CONFIG_FILENAME);

        Self::read_from_file(file_path)
    }

    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = fs::read(path)?;

        let mut config: Self = toml::from_slice(&contents).map_err(|source| ConfigError::Toml {
            source,
            path: path.to_owned(),
        })?;
        config.file_path = path.to_owned();
        config.make_paths_absolute();

        Ok(config)
    }

    /// The path that paths in this Config should be considered relative to.
    pub fn folder(&self) -> &Path {
        self.file_path.parent().unwrap()
    }

    /// Turn all relative paths referenced from this config into absolute paths.
    fn make_paths_absolute(&mut self) {
        let base = self.file_path.parent().unwrap();

        if let Some(list_path) = self.asset_list_path.as_mut() {
            make_absolute(list_path, base);
        }

        if let Some(cache_path) = self.asset_cache_path.as_mut() {
            make_absolute(cache_path, base);
        }

        for include in &mut self.includes {
            make_absolute(include, base);
        }

        for input in &mut self.inputs {
            if let Some(codegen_path) = input.codegen_path.as_mut() {
                make_absolute(codegen_path, base);
            }

            make_absolute(&mut input.base_path, base);
        }
    }
}

fn default_max_spritesheet_size() -> (u32, u32) {
    (1024, 1024)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct InputConfig {
    /// A glob that will match all files that should be considered for this
    /// group of inputs.
    pub glob: Glob,

    /// Defines whether Tarmac should generate code to import the assets
    /// associated with this group of inputs.
    #[serde(default)]
    pub codegen: bool,

    /// If specified, batches together all of the generated code for this group
    /// of inputs into a single file created at this path.
    #[serde(default)]
    pub codegen_path: Option<PathBuf>,

    #[serde(default)]
    pub base_path: PathBuf,

    /// Whether the assets affected by this config are allowed to be packed into
    /// spritesheets.
    ///
    /// This isn't enabled by default because special considerations need to be
    /// made in order to correctly handle spritesheets. Not all images are able
    /// to be pre-packed into spritesheets, like images used in `Decal`
    /// instances.
    #[serde(default)]
    pub packable: bool,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Error deserializing TOML from path {}", .path.display())]
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },
}

impl ConfigError {
    /// Tells whether this ConfigError originated because of a path not
    /// existing.
    ///
    /// This is intended for use with methods like `Config::read_from_folder` in
    /// order to avoid needing to check if a file with the right name exists.
    pub fn is_not_found(&self) -> bool {
        match self {
            ConfigError::Io { source } => source.kind() == io::ErrorKind::NotFound,
            _ => false,
        }
    }
}

/// Utility to make a path absolute if it is not absolute already.
fn make_absolute(path: &mut PathBuf, base: &Path) {
    if path.is_relative() {
        let new_path = base.join(&*path);
        *path = new_path;
    }
}
