use std::{
    env, fs,
    path::{self, Path, PathBuf},
};

use sha2::{Digest, Sha256};
use snafu::ResultExt;

use crate::{
    auth_cookie::get_auth_cookie,
    config::{Config, ConfigEntry},
    manifest::{Manifest, ManifestAsset},
    options::{GlobalOptions, SyncOptions, SyncTarget},
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

mod error {
    use crate::{config::ConfigError, manifest::ManifestError};
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
    let current_dir = env::current_dir().context(error::CurrentDir)?;
    let mut session = SyncSession::new(&current_dir)?;

    // If the user specified no paths, use the current working directory as the
    // input path.
    let paths = if options.paths.is_empty() {
        vec![current_dir.clone()]
    } else {
        options.paths
    };

    for path in paths {
        session.feed(&current_dir, &path)?;
    }

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

    Ok(())
}

/// A sync session holds all of the state for a single run of the 'tarmac sync'
/// command.
#[derive(Debug)]
struct SyncSession {
    /// The path where this sync session was started from.
    /// $root_path/tarmac-manifest.toml will be updated when the sync session is
    /// over.
    root_path: PathBuf,

    /// The contents of the original manifest from the session's root path, or
    /// the default value if it wasn't present.
    source_manifest: Manifest,

    /// All of the assets loaded into the sync session so far.
    assets: Vec<SyncAsset>,
}

#[derive(Debug)]
struct SyncAsset {
    /// The absolute path to the asset on disk.
    path: PathBuf,

    /// The cleaned up, platform-independent path to this asset, relative to the
    /// sync session's root path.
    ///
    /// This path will be used as a key into the Tarmac manifest and is used
    /// across runs to preserve the identity of an asset.
    display_path: String,

    /// Hierarchical configuration picked up when discovering this asset.
    ///
    /// Configuration is defined in tarmac.toml files.
    config: ConfigEntry,

    /// The current manifest data for this asset. Will be loaded from the sync
    /// session's source manifest if an entry exists, or set to the default and
    /// updated as part of the sync process.
    manifest_entry: ManifestAsset,
}

impl SyncSession {
    fn new(root_path: &Path) -> Result<Self, SyncError> {
        log::trace!("Starting new sync session");

        let source_manifest = Manifest::read_from_folder(root_path)
            .context(error::Manifest)?
            .unwrap_or_default();

        Ok(Self {
            root_path: root_path.to_path_buf(),
            source_manifest,
            assets: Vec::new(),
        })
    }

    /// Load a path into the sync session as a source of asset files.
    fn feed(&mut self, root_path: &Path, path: &Path) -> Result<(), SyncError> {
        log::trace!("Feeding path to sync session: {}", path.display());

        let config = Self::find_config(path)?.unwrap_or_default();
        self.feed_inner(root_path, path, config.default)
    }

    /// Recursive implementation function for `feed`
    fn feed_inner(
        &mut self,
        root_path: &Path,
        path: &Path,
        current_config: ConfigEntry,
    ) -> Result<(), SyncError> {
        let meta = fs::metadata(path).context(error::Io { path })?;

        if meta.is_file() {
            if is_image_asset(path) {
                let display_path = spruce_up_path(root_path, path);

                let manifest_entry = self
                    .source_manifest
                    .assets
                    .get(&display_path)
                    .cloned()
                    .unwrap_or_default();

                log::trace!("Adding asset {}", display_path);
                self.assets.push(SyncAsset {
                    path: path.to_path_buf(),
                    display_path,
                    config: current_config,
                    manifest_entry,
                });
            }
        } else {
            // If this is a folder, it's possible that it contains a config. We
            // should read it and apply to all of its descendants until we find
            // a new config file.
            let child_config = Config::read_from_folder(path)
                .context(error::Config)?
                .map(|config| {
                    log::trace!("Found config in {}", path.display());

                    config.default
                })
                .unwrap_or(current_config);

            let children = fs::read_dir(path).context(error::Io { path })?;
            for child_entry in children {
                let child_entry = child_entry.context(error::Io { path })?;

                self.feed_inner(root_path, &child_entry.path(), child_config.clone())?;
            }
        }

        Ok(())
    }

    /// Attempt to locate a config in the given path or any of its ancestors.
    fn find_config(path: &Path) -> Result<Option<Config>, SyncError> {
        let meta = fs::metadata(path).context(error::Io { path })?;

        if meta.is_dir() {
            if let Some(config) = Config::read_from_folder(path).context(error::Config)? {
                return Ok(Some(config));
            }
        }

        if let Some(parent) = path.parent() {
            Self::find_config(parent)
        } else {
            Ok(None)
        }
    }

    fn sync_to_roblox(&mut self, auth: String) -> Result<(), SyncError> {
        let mut api_client = RobloxApiClient::new(auth);

        for asset in &mut self.assets {
            let asset_content = fs::read(&asset.path).context(error::Io { path: &asset.path })?;
            let asset_hash = generate_asset_hash(&asset_content);

            let need_to_upload = if asset.manifest_entry.uploaded_id.is_some() {
                // If this asset has been uploaded before, compare the content
                // hash to see if it's changed since the most recent upload.
                match &asset.manifest_entry.uploaded_hash {
                    Some(existing_hash) => existing_hash != &asset_hash,
                    None => true,
                }
            } else {
                // If we haven't uploaded this asset before, we definitely need
                // to upload it.
                true
            };

            if need_to_upload {
                println!("Uploading {}", asset.display_path);

                let name = asset.path.file_stem().unwrap().to_str().unwrap();

                let response = api_client
                    .upload_image(ImageUploadData {
                        image_data: asset_content,
                        name,
                        description: "Uploaded by Tarmac.",
                    })
                    .expect("Upload failed");

                asset.manifest_entry.uploaded_id = Some(response.backing_asset_id);
                asset.manifest_entry.uploaded_hash = Some(asset_hash);
            }
        }

        Ok(())
    }

    fn sync_to_content_folder(&mut self) -> Result<(), SyncError> {
        unimplemented!("TODO: Implement syncing to the content folder");
    }

    fn write_manifest(&self) -> Result<(), SyncError> {
        let manifest = Manifest::from_assets(
            self.assets
                .iter()
                .map(|asset| (asset.display_path.clone(), asset.manifest_entry.clone())),
        );

        manifest
            .write_to_folder(&self.root_path)
            .context(error::Manifest)?;

        Ok(())
    }
}

fn spruce_up_path(root_path: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root_path).unwrap();
    let displayed = format!("{}", relative.display());

    // In order to make relative paths behave cross-platform, fix the path
    // separator to always be / on platforms where it isn't the main separator.
    if path::MAIN_SEPARATOR == '/' {
        displayed
    } else {
        displayed.replace(path::MAIN_SEPARATOR, "/")
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
