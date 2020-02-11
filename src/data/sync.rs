use std::path::PathBuf;

use crate::data::{ImageSlice, InputConfig, InputManifest};

/// In-memory representation of a Tarmac Input during the sync process.
///
/// SyncInput structs are gradually created and filled in from the filesystem,
/// results of network I/O, and from the previous Tarmac manifest file.
#[derive(Debug)]
pub struct SyncInput {
    /// The path on disk to the file this input originated from.
    pub path: PathBuf,

    /// The configuration that applied to this input when it was discovered.
    pub config: InputConfig,

    /// The contents of the file this input originated from.
    pub contents: Vec<u8>,

    /// A hash of `contents`.
    pub hash: String,

    /// If this input has been part of an upload to Roblox.com, contains the
    /// asset ID that contains the data from this input.
    pub id: Option<u64>,

    /// If this input has been packed into a spritesheet, contains the slice of
    /// the spritesheet that this input is located in.
    pub slice: Option<ImageSlice>,
}

impl SyncInput {
    pub fn is_unchanged_since_last_sync(&self, old_manifest: &InputManifest) -> bool {
        self.hash == old_manifest.hash && self.config.packable == old_manifest.packable
    }
}
