use std::{
    env, fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use snafu::{IntoError, ResultExt};

use crate::{
    asset_name::AssetName,
    auth_cookie::get_auth_cookie,
    data::{CodegenKind, Manifest, ManifestError, ProjectConfig, ProjectConfigError},
    options::{GlobalOptions, SyncOptions, SyncTarget},
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

mod error {
    use crate::data::{ManifestError, ProjectConfigError};
    use snafu::Snafu;
    use std::{io, path::PathBuf};

    #[derive(Debug, Snafu)]
    #[snafu(visibility = "pub(super)")]
    pub enum SyncError {
        #[snafu(display("{}", source))]
        ProjectConfig {
            source: ProjectConfigError,
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
    let fuzzy_project_path = match options.project_path {
        Some(v) => v,
        None => env::current_dir().context(error::CurrentDir)?,
    };

    let mut session = SyncSession::new(&fuzzy_project_path)?;

    match options.target {
        SyncTarget::Roblox => {
            let auth = global
                .auth
                .or_else(get_auth_cookie)
                .expect("no auth cookie found");

            session.sync_to_roblox(auth)?;
        }
        SyncTarget::ContentFolder => session.sync_to_content_folder()?,
    }

    log::trace!("Session: {:#?}", session);

    session.write_manifest()?;
    // session.codegen()?;

    Ok(())
}

/// A sync session holds all of the state for a single run of the 'tarmac sync'
/// command.
#[derive(Debug)]
struct SyncSession {
    project: ProjectConfig,

    original_manifest: Manifest,

    /// The path where this sync session was started from.
    /// $root_path/tarmac-manifest.toml will be updated when the sync session is
    /// over.
    root_path: PathBuf,
}

impl SyncSession {
    fn new(fuzzy_project_path: &Path) -> Result<Self, SyncError> {
        log::trace!("Starting new sync session");

        let project = ProjectConfig::read_from_folder_or_file(&fuzzy_project_path)
            .context(error::ProjectConfig)?;

        let root_path = project.file_path.parent().unwrap().to_owned();

        let original_manifest = match Manifest::read_from_folder(&root_path) {
            Ok(manifest) => manifest,
            Err(err) if err.is_not_found() => Manifest::default(),
            other => other.context(error::Manifest)?,
        };

        Ok(Self {
            project,
            original_manifest,
            root_path,
        })
    }

    fn sync_to_roblox(&mut self, auth: String) -> Result<(), SyncError> {
        let mut api_client = RobloxApiClient::new(auth);

        Ok(())
    }

    fn sync_to_content_folder(&mut self) -> Result<(), SyncError> {
        unimplemented!("TODO: Implement syncing to the content folder");
    }

    fn write_manifest(&self) -> Result<(), SyncError> {
        // let manifest = Manifest::from_assets(
        //     self.assets
        //         .iter()
        //         .map(|asset| (asset.name.clone(), asset.manifest_entry.clone())),
        // );

        // manifest
        //     .write_to_folder(&self.root_path)
        //     .context(error::Manifest)?;

        Ok(())
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
