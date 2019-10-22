use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    assets: HashMap<String, ManifestAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestAsset {
    uploaded_id: Option<u64>,
    uploaded_hash: Option<String>,
}
