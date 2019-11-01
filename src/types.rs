use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::asset_name::AssetName;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupConfig {
    paths: Vec<String>,
    // TODO: ignore globs?
    // TODO: input globs instead of paths?
    spritesheet: Option<SpritesheetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpritesheetConfig {
    max_size: (usize, usize),
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
struct InputConfig {
    codegen: (),
    can_spritesheet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Group {
    config: GroupConfig,
    inputs: HashSet<AssetName>,
    outputs: HashSet<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Output {
    uploaded_id: u64,
    uploaded_hash: String,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    groups: HashMap<String, GroupConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    groups: HashMap<String, Group>,
    inputs: HashMap<AssetName, InputConfig>,
    outputs: HashMap<u64, Output>,
}
