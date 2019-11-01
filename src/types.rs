use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::asset_name::AssetName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub paths: Vec<String>,
    // TODO: ignore globs?
    // TODO: input globs instead of paths?
    pub spritesheet: Option<SpritesheetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpritesheetConfig {
    pub max_size: (usize, usize),
    // TODO: packing algorithm?
}

impl Default for SpritesheetConfig {
    fn default() -> Self {
        Self {
            max_size: (1024, 1024),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub codegen: (),
    pub can_spritesheet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub config: GroupConfig,
    pub inputs: HashSet<AssetName>,
    pub outputs: HashSet<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub uploaded_id: u64,
    pub uploaded_hash: String,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub groups: HashMap<String, GroupConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub groups: HashMap<String, Group>,
    pub inputs: HashMap<AssetName, InputConfig>,
    pub outputs: HashMap<u64, Output>,
}
