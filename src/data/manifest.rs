use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{GroupConfig, InputConfig};
use crate::asset_name::AssetName;

static MANIFEST_FILENAME: &str = "tarmac-manifest.toml";

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
