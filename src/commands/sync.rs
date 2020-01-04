use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    env, fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use snafu::ResultExt;

use crate::{
    asset_name::AssetName,
    auth_cookie::get_auth_cookie,
    data::{Config, Manifest},
    options::{GlobalOptions, SyncOptions, SyncTarget},
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

mod error {
    use crate::data::{ConfigError, ManifestError};
    use snafu::Snafu;
    use std::{io, path::PathBuf};

    #[derive(Debug, Snafu)]
    #[snafu(visibility = "pub(super)")]
    pub enum SyncError {
        #[snafu(display("{}", source))]
        Config {
            source: ConfigError,
        },

        #[snafu(display("{}", source))]
        Manifest {
            source: ManifestError,
        },

        Io {
            path: PathBuf,
            source: io::Error,
        },

        #[snafu(display("couldn't get the current directory of the process"))]
        CurrentDir {
            source: io::Error,
        },

        #[snafu(display("'tarmac sync' requires an authentication cookie"))]
        NoAuth,
    }
}

pub use error::SyncError;

pub fn sync(global: GlobalOptions, options: SyncOptions) -> Result<(), SyncError> {
    let fuzzy_config_path = match options.config_path {
        Some(v) => v,
        None => env::current_dir().context(error::CurrentDir)?,
    };

    let mut session = SyncSession::new(&fuzzy_config_path)?;

    // session.gather_inputs()?;

    match options.target {
        SyncTarget::Roblox => {
            let auth = global
                .auth
                .or_else(get_auth_cookie)
                .expect("no auth cookie found");

            // session.sync_to_roblox(auth)?;
        }
        SyncTarget::ContentFolder => {
            // session.sync_to_content_folder()?;
        }
    }

    // session.write_manifest()?;
    // session.codegen()?;

    Ok(())
}

/// A sync session holds all of the state for a single run of the 'tarmac sync'
/// command.
#[derive(Debug)]
struct SyncSession {
    /// The config file pulled from the starting point of the sync operation.
    root_config: Config,

    /// The manifest file that was present as of the beginning of the sync
    /// operation.
    original_manifest: Manifest,
}

impl SyncSession {
    fn new(fuzzy_config_path: &Path) -> Result<Self, SyncError> {
        log::trace!("Starting new sync session");

        let root_config =
            Config::read_from_folder_or_file(&fuzzy_config_path).context(error::Config)?;

        let original_manifest = match Manifest::read_from_folder(root_config.folder()) {
            Ok(manifest) => manifest,
            Err(err) if err.is_not_found() => Manifest::default(),
            other => other.context(error::Manifest)?,
        };

        Ok(Self {
            root_config,
            original_manifest,
        })
    }
}

fn is_image_asset(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        // TODO: Expand the definition of images?
        Some("png") | Some("jpg") => true,

        _ => false,
    }
}

fn generate_asset_hash(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}
