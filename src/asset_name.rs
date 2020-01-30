use std::{
    fmt,
    path::{self, Path},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

/// Represents a disambiguated and cleaned up path to an asset from a Tarmac
/// project.
///
/// This is really just a string, but by making it have an explicit type with
/// known conversions, we can avoid some kinds of error trying to use Tarmac
/// APIs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetName(Arc<str>);

impl AssetName {
    pub fn from_paths(root_path: &Path, asset_path: &Path) -> Self {
        let relative = asset_path
            .strip_prefix(root_path)
            .expect("AssetName::from_paths expects asset_path to have root_path as a prefix.");

        let displayed = format!("{}", relative.display());

        // In order to make relative paths behave cross-platform, fix the path
        // separator to always be / on platforms where it isn't the main separator.
        let displayed = if path::MAIN_SEPARATOR == '/' {
            displayed
        } else {
            displayed.replace(path::MAIN_SEPARATOR, "/")
        };

        AssetName(displayed.into())
    }

    /// Spritesheets don't have any canonical name provided by Tarmac's inputs;
    /// when we upload them, we want to give them a simple dummy name
    pub fn spritesheet() -> Self {
        AssetName("spritesheet.png".into())
    }
}

impl AsRef<str> for AssetName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetName {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
