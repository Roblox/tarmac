use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    env, fs, iter,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use snafu::ResultExt;
use walkdir::WalkDir;

use crate::{
    asset_name::AssetName,
    auth_cookie::get_auth_cookie,
    data::{Config, InputConfig, Manifest},
    options::{GlobalOptions, SyncOptions, SyncTarget},
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

use self::error::Error;
pub use self::error::Error as SyncError;

pub fn sync(global: GlobalOptions, options: SyncOptions) -> Result<(), Error> {
    let fuzzy_config_path = match options.config_path {
        Some(v) => v,
        None => env::current_dir().context(error::CurrentDir)?,
    };

    let mut api_client = global
        .auth
        .or_else(get_auth_cookie)
        .map(|auth| RobloxApiClient::new(auth));

    let mut session = SyncSession::new(&fuzzy_config_path)?;

    session.discover_configs()?;
    session.discover_inputs()?;

    match options.target {
        SyncTarget::Roblox => {
            let api_client = api_client.as_mut().ok_or(Error::NoAuth)?;
            let mut strategy = RobloxUploadStrategy { api_client };

            session.sync(&mut strategy)?;
        }
        SyncTarget::ContentFolder => {
            let mut strategy = ContentUploadStrategy {};

            session.sync(&mut strategy)?;
        }
    }

    session.write_manifest()?;
    session.codegen()?;

    Ok(())
}

/// A sync session holds all of the state for a single run of the 'tarmac sync'
/// command.
#[derive(Debug)]
struct SyncSession {
    /// The config file pulled from the starting point of the sync operation.
    root_config: Config,

    /// Config files discovered by searching through the `includes` section of
    /// known config files, recursively.
    non_root_configs: Vec<Config>,

    /// The manifest file that was present as of the beginning of the sync
    /// operation.
    original_manifest: Manifest,

    /// All of the inputs discovered so far in the current sync.
    inputs: HashMap<AssetName, SyncInput>,
}

#[derive(Debug)]
struct SyncInput {
    path: PathBuf,
    config: InputConfig,
}

impl SyncSession {
    fn new(fuzzy_config_path: &Path) -> Result<Self, Error> {
        log::trace!("Starting new sync session");

        let root_config =
            Config::read_from_folder_or_file(&fuzzy_config_path).context(error::Config)?;

        log::trace!("Starting from config \"{}\"", root_config.name);

        let original_manifest = match Manifest::read_from_folder(root_config.folder()) {
            Ok(manifest) => manifest,
            Err(err) if err.is_not_found() => Manifest::default(),
            other => other.context(error::Manifest)?,
        };

        Ok(Self {
            root_config,
            non_root_configs: Vec::new(),
            original_manifest,
            inputs: Default::default(),
        })
    }

    /// Locate all of the configs connected to our root config.
    ///
    /// Tarmac config files can include eachother via the `includes` field,
    /// which will search the given path for other config files and use them as
    /// part of the sync.
    fn discover_configs(&mut self) -> Result<(), Error> {
        let mut to_search = VecDeque::new();
        to_search.extend(
            self.root_config
                .includes
                .iter()
                .map(|include| include.path.clone()),
        );

        while let Some(search_path) = to_search.pop_front() {
            let search_meta =
                fs::metadata(&search_path).context(error::Io { path: &search_path })?;

            if search_meta.is_file() {
                // This is a file that's explicitly named by a config. We'll
                // check that it's a Tarmac config and include it.

                let config = Config::read_from_file(&search_path).context(error::Config)?;

                // Include any configs that this config references.
                to_search.extend(config.includes.iter().map(|include| include.path.clone()));

                self.non_root_configs.push(config);
            } else {
                // If this directory contains a config file, we can stop
                // traversing this branch.

                match Config::read_from_folder(&search_path) {
                    Ok(config) => {
                        // We found a config, we're done here.

                        // Append config include paths from this config
                        to_search
                            .extend(config.includes.iter().map(|include| include.path.clone()));

                        self.non_root_configs.push(config);
                    }

                    Err(err) if err.is_not_found() => {
                        // We didn't find a config, keep searching down this
                        // branch of the filesystem.

                        let children =
                            fs::read_dir(&search_path).context(error::Io { path: &search_path })?;

                        for entry in children {
                            let entry = entry.context(error::Io { path: &search_path })?;
                            let entry_path = entry.path();

                            // DirEntry has a metadata method, but in the case
                            // of symlinks, it returns metadata about the
                            // symlink and not the file or folder.
                            let entry_meta = fs::metadata(&entry_path)
                                .context(error::Io { path: &entry_path })?;

                            if entry_meta.is_dir() {
                                to_search.push_back(entry_path);
                            }
                        }
                    }

                    err @ Err(_) => {
                        err.context(error::Config)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Find all files on the filesystem referenced as inputs by our configs.
    fn discover_inputs(&mut self) -> Result<(), Error> {
        let inputs = &mut self.inputs;

        // Starting with our root config, iterate over all configs and find all
        // relevant inputs
        for config in iter::once(&self.root_config).chain(self.non_root_configs.iter()) {
            let config_path = config.folder();

            for input_config in &config.inputs {
                let base_path = config_path.join(input_config.glob.get_prefix());
                log::trace!(
                    "Searching for inputs in '{}' matching '{}'",
                    base_path.display(),
                    input_config.glob,
                );

                let filtered_paths = WalkDir::new(base_path)
                    .into_iter()
                    // TODO: Properly handle WalkDir errors
                    .filter_map(Result::ok)
                    .filter(|entry| {
                        let match_path = entry.path().strip_prefix(config_path).unwrap();
                        input_config.glob.is_match(match_path)
                    });

                for matching in filtered_paths {
                    let name = AssetName::from_paths(config_path, matching.path());
                    log::trace!("Found input {}", name);

                    let already_found = inputs.insert(
                        name,
                        SyncInput {
                            path: matching.into_path(),
                            config: input_config.clone(),
                        },
                    );

                    if let Some(existing) = already_found {
                        return Err(Error::OverlappingGlobs {
                            path: existing.path,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn sync<S: UploadStrategy>(&mut self, strategy: &mut S) -> Result<(), Error> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        struct InputCompatibility {
            packable: bool,
        }

        let mut compatible_input_groups = HashMap::new();

        for (input_name, input) in &self.inputs {
            let compatibility = InputCompatibility {
                packable: input.config.packable,
            };

            let input_group = compatible_input_groups
                .entry(compatibility)
                .or_insert_with(Vec::new);

            input_group.push(input_name.clone());
        }

        for (compatibility, group) in compatible_input_groups {
            if compatibility.packable {
                log::warn!("TODO: Support packing images");
            } else {
                for input_name in group {
                    let input = self.inputs.get(&input_name).unwrap();

                    if is_image_asset(&input.path) {
                        self.sync_unpackable_image(strategy, &input_name)?;
                    } else {
                        log::warn!("Didn't know what to do with asset {}", input.path.display());
                    }
                }
            }
        }

        Ok(())
    }

    fn sync_unpackable_image<S: UploadStrategy>(
        &mut self,
        strategy: &mut S,
        input_name: &AssetName,
    ) -> Result<(), Error> {
        let input = self.inputs.get(input_name).unwrap();
        let contents = fs::read(&input.path).context(error::Io { path: &input.path })?;
        let hash = generate_asset_hash(&contents);

        let upload_data = UploadData {
            name: input_name.clone(),
            contents,
            hash: hash.clone(),
        };

        match self.original_manifest.inputs.get(&input_name) {
            Some(input_manifest) => {
                match &input_manifest.hash {
                    Some(prev_hash) => {
                        if &hash != prev_hash {
                            strategy.upload(upload_data)?;
                            return Ok(());
                        }
                    }
                    None => {
                        strategy.upload(upload_data)?;
                        return Ok(());
                    }
                }

                if input_manifest.id.is_none() {
                    strategy.upload(upload_data)?;
                    return Ok(());
                }

                let prev_config = self.original_manifest.configs.get(&input_manifest.config);

                match prev_config {
                    Some(prev_config) => {
                        if prev_config != &input.config {
                            strategy.upload(upload_data)?;
                            return Ok(());
                        }
                    }
                    None => {
                        strategy.upload(upload_data)?;
                        return Ok(());
                    }
                }
            }
            None => {
                strategy.upload(upload_data)?;
                return Ok(());
            }
        }

        Ok(())
    }

    fn write_manifest(&self) -> Result<(), Error> {
        // TODO: Generate a new manifest based on our current inputs and write
        // it to disk.

        Ok(())
    }

    fn codegen(&self) -> Result<(), Error> {
        // TODO: For each input, use its config to write a file pointing to
        // where the asset ended up.

        Ok(())
    }
}

struct UploadResponse {
    id: u64,
    // TODO: Other asset URL construction information to support content folder
    // shenanigans.
}

struct UploadData {
    name: AssetName,
    contents: Vec<u8>,
    hash: String,
}

trait UploadStrategy {
    fn upload(&mut self, data: UploadData) -> Result<UploadResponse, SyncError>;
}

struct RobloxUploadStrategy<'a> {
    api_client: &'a mut RobloxApiClient,
}

impl<'a> UploadStrategy for RobloxUploadStrategy<'a> {
    fn upload(&mut self, data: UploadData) -> Result<UploadResponse, SyncError> {
        log::info!("Uploading {}", &data.name);

        let response = self
            .api_client
            .upload_image(ImageUploadData {
                image_data: Cow::Owned(data.contents),
                name: data.name.as_ref(),
                description: "Uploaded by Tarmac.",
            })
            .expect("Upload failed");

        log::info!(
            "Uploaded {} to ID {}",
            &data.name,
            response.backing_asset_id
        );

        Ok(UploadResponse {
            id: response.backing_asset_id,
        })
    }
}

struct ContentUploadStrategy {
    // TODO: Studio install information
}

impl UploadStrategy for ContentUploadStrategy {
    fn upload(&mut self, _data: UploadData) -> Result<UploadResponse, SyncError> {
        unimplemented!("content folder uploading");
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

mod error {
    use crate::data::{ConfigError, ManifestError};
    use snafu::Snafu;
    use std::{io, path::PathBuf};
    use walkdir;

    #[derive(Debug, Snafu)]
    #[snafu(visibility = "pub(super)")]
    pub enum Error {
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

        // TODO: Add more detail here and better display
        #[snafu(display("{}", source))]
        WalkDir {
            source: walkdir::Error,
        },

        // TODO: Add more detail here and better display
        #[snafu(display("Path {} was described by more than one glob", path.display()))]
        OverlappingGlobs {
            path: PathBuf,
        },
    }
}
