use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    default: ConfigEntry,

    #[serde(flatten)]
    paths: HashMap<String, ConfigEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigEntry {
    codegen: CodegenKind,
    can_spritesheet: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CodegenKind {
    AssetUrl,
}
