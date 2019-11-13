use std::{
    collections::{BTreeSet, HashMap},
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use super::{GroupConfig, InputConfig};
use crate::asset_name::AssetName;

static MANIFEST_FILENAME: &str = "tarmac-manifest.toml";

/// Tracks the status of all groups, inputs, and outputs as of the last Tarmac
/// sync.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub groups: HashMap<String, GroupManifest>,
    pub inputs: HashMap<AssetName, InputManifest>,
    pub outputs: HashMap<u64, OutputManifest>,
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

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupManifest {
    /// The configuration defined in a tarmac-project.toml that created this
    /// group.
    pub config: GroupConfig,

    /// All of the paths that were part of this group last time any sync was
    /// run.
    pub inputs: BTreeSet<AssetName>,

    /// All of the assets that this group turned into the last time it was
    /// uploaded.
    pub outputs: BTreeSet<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputManifest {
    /// The hierarchical config applied to this config the last time it was part
    /// of an upload.
    pub uploaded_config: Option<InputConfig>,

    /// The hexadecimal encoded hash of the contents of this input the last time
    /// it was part of an upload.
    pub uploaded_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputManifest {
    /// The asset ID on Roblox.com that this asset was uploaded to the last time
    /// it was part of an upload.
    pub uploaded_id: u64,
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
