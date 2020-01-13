use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use crate::{asset_name::AssetName, data::config::CodegenKind};

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

        let contents = fs::read(file_path).context(Io { file_path })?;
        let config = toml::from_slice(&contents).context(DeserializeToml { file_path })?;

        Ok(config)
    }

    pub fn write_to_folder<P: AsRef<Path>>(&self, folder_path: P) -> Result<(), ManifestError> {
        let folder_path = folder_path.as_ref();
        let file_path = &folder_path.join(MANIFEST_FILENAME);

        let serialized = toml::to_vec(self).context(SerializeToml)?;
        fs::write(file_path, serialized).context(Io { file_path })?;

        log::trace!("Saved manifest to {}", file_path.display());

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct InputManifest {
    /// The hexadecimal encoded hash of the contents of this input the last time
    /// it was part of an upload.
    pub hash: Option<String>,

    /// The asset ID that contains this input the last time it was uploaded.
    pub id: Option<u64>,

    /// If the asset is an image that was packed into a spritesheet, contains
    /// the portion of the uploaded image that contains this input.
    pub slice: Option<ImageSlice>,

    /// Whether the config applied to this input asked for it to be packed into
    /// a spritesheet.
    pub packable: bool,

    /// The kind of Lua code that was generated during the last sync for this
    /// input.
    pub codegen: CodegenKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSlice {
    pub min: (u32, u32),
    pub max: (u32, u32),
}

#[derive(Debug, Snafu)]
pub enum ManifestError {
    DeserializeToml {
        file_path: PathBuf,
        source: toml::de::Error,
    },

    SerializeToml {
        source: toml::ser::Error,
    },

    Io {
        file_path: PathBuf,
        source: io::Error,
    },
}

impl ManifestError {
    pub fn is_not_found(&self) -> bool {
        match self {
            ManifestError::Io { source, .. } => source.kind() == io::ErrorKind::NotFound,
            _ => false,
        }
    }
}
