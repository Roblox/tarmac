use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};

use fs_err as fs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::asset_name::AssetName;

static MANIFEST_FILENAME: &str = "tarmac-manifest.toml";

/// Tracks the status of all configuration, inputs, and outputs as of the last
/// sync operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub inputs: BTreeMap<AssetName, InputManifest>,
}

impl Manifest {
    pub fn read_from_folder<P: AsRef<Path>>(folder_path: P) -> Result<Self, ManifestError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(MANIFEST_FILENAME);

        let contents = fs::read(file_path)?;
        let config =
            toml::from_slice(&contents).map_err(|source| ManifestError::DeserializeToml {
                source,
                file_path: file_path.to_owned(),
            })?;

        Ok(config)
    }

    pub fn write_to_folder<P: AsRef<Path>>(&self, folder_path: P) -> Result<(), ManifestError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(MANIFEST_FILENAME);

        let serialized = toml::to_vec(self)?;
        fs::write(file_path, serialized)?;

        log::trace!("Saved manifest to {}", file_path.display());

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct InputManifest {
    /// The hexadecimal encoded hash of the contents of this input the last time
    /// it was part of an upload.
    pub hash: String,

    /// The asset ID that contains this input the last time it was uploaded.
    pub id: Option<u64>,

    /// If the asset is an image that was packed into a spritesheet, contains
    /// the portion of the uploaded image that contains this input.
    pub slice: Option<ImageSlice>,

    /// Whether the config applied to this input asked for it to be packed into
    /// a spritesheet.
    pub packable: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImageSlice {
    coordinates: ((u32, u32), (u32, u32)),
}

impl ImageSlice {
    pub fn new(min: (u32, u32), max: (u32, u32)) -> Self {
        Self {
            coordinates: (min, max),
        }
    }

    pub fn min(&self) -> (u32, u32) {
        self.coordinates.0
    }

    pub fn max(&self) -> (u32, u32) {
        self.coordinates.1
    }

    pub fn size(&self) -> (u32, u32) {
        let (x1, y1) = self.min();
        let (x2, y2) = self.max();

        (x2 - x1, y2 - y1)
    }
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("Error deserializing TOML from path {}", .file_path.display())]
    DeserializeToml {
        file_path: PathBuf,
        source: toml::de::Error,
    },

    #[error(transparent)]
    SerializeToml {
        #[from]
        source: toml::ser::Error,
    },

    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },
}

impl ManifestError {
    pub fn is_not_found(&self) -> bool {
        match self {
            ManifestError::Io { source } => source.kind() == io::ErrorKind::NotFound,
            _ => false,
        }
    }
}
