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
    roblox_web_api::RobloxApiClient,
};

mod error {
    use crate::data::{ConfigError, ManifestError};
    use snafu::Snafu;
    use std::{io, path::PathBuf};
    use walkdir;

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

pub use error::SyncError;

pub fn sync(global: GlobalOptions, options: SyncOptions) -> Result<(), SyncError> {
    let fuzzy_config_path = match options.config_path {
        Some(v) => v,
        None => env::current_dir().context(error::CurrentDir)?,
    };

    let api_client = global
        .auth
        .or_else(get_auth_cookie)
        .map(|auth| RobloxApiClient::new(auth));

    let mut session = SyncSession::new(api_client, &fuzzy_config_path)?;

    session.discover_configs()?;
    session.discover_inputs()?;

    match options.target {
        SyncTarget::Roblox => session.sync_to_roblox()?,
        SyncTarget::ContentFolder => session.sync_to_content_folder()?,
    }

    session.write_manifest()?;
    session.codegen()?;

    Ok(())
}

/// A sync session holds all of the state for a single run of the 'tarmac sync'
/// command.
#[derive(Debug)]
struct SyncSession {
    /// If available, represents the handle to the Roblox web API.
    api_client: Option<RobloxApiClient>,

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
    fn new(
        api_client: Option<RobloxApiClient>,
        fuzzy_config_path: &Path,
    ) -> Result<Self, SyncError> {
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
            api_client,
            root_config,
            non_root_configs: Vec::new(),
            original_manifest,
            inputs: Default::default(),
        })
    }

    fn discover_configs(&mut self) -> Result<(), SyncError> {
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
                to_search.extend(config.includes.iter().map(|include| include.path.clone()));
                self.non_root_configs.push(config);
            } else {
                // If this directory contains a config file, we can stop
                // traversing this branch.

                match Config::read_from_folder(&search_path) {
                    // We found a config, we're done here
                    Ok(config) => {
                        to_search
                            .extend(config.includes.iter().map(|include| include.path.clone()));
                        self.non_root_configs.push(config);
                    }

                    // We didn't find a config, keep searching
                    Err(err) if err.is_not_found() => {
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

                    // We hit some other error, cascade it upwards
                    err @ Err(_) => {
                        err.context(error::Config)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn discover_inputs(&mut self) -> Result<(), SyncError> {
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
                    if let Some(existing) = inputs.insert(
                        name,
                        SyncInput {
                            path: matching.into_path(),
                            config: input_config.clone(),
                        },
                    ) {
                        return Err(SyncError::OverlappingGlobs {
                            path: existing.path,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn sync_to_roblox(&mut self) -> Result<(), SyncError> {
        let _client = self.api_client.as_mut().ok_or(SyncError::NoAuth)?;

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
                        self.sync_unpackable_image(&input_name)?;
                    } else {
                        log::warn!("Didn't know what to do with asset {}", input.path.display());
                    }
                }
            }
        }

        Ok(())
    }

    fn sync_to_content_folder(&mut self) -> Result<(), SyncError> {
        Ok(())
    }

    fn sync_unpackable_image(&mut self, input_name: &AssetName) -> Result<(), SyncError> {
        let input = self.inputs.get(input_name).unwrap();
        let mut contents = LazyFileContents::new(&input.path);

        match self.original_manifest.inputs.get(&input_name) {
            Some(input_manifest) => {
                match &input_manifest.hash {
                    Some(prev_hash) => {
                        let hash = contents.hash()?;

                        if hash != prev_hash {
                            // upload this!
                            return self.upload_unpacked_image();
                        }
                    }
                    None => {
                        // upload this!
                        return self.upload_unpacked_image();
                    }
                }

                if input_manifest.id.is_none() {
                    // upload this.
                    return self.upload_unpacked_image();
                }

                let prev_config = self.original_manifest.configs.get(&input_manifest.config);

                match prev_config {
                    Some(prev_config) => {
                        if prev_config != &input.config {
                            // upload!
                        }
                    }
                    None => {
                        // malformed manifest, let's upload.
                        return self.upload_unpacked_image();
                    }
                }
            }
            None => {
                // upload this.
                return self.upload_unpacked_image();
            }
        }

        Ok(())
    }

    fn upload_unpacked_image(&mut self) -> Result<(), SyncError> {
        // TODO

        Ok(())
    }

    fn write_manifest(&self) -> Result<(), SyncError> {
        // TODO: Generate a new manifest based on our current inputs and write
        // it to disk.

        Ok(())
    }

    fn codegen(&self) -> Result<(), SyncError> {
        // TODO: For each input, use its config to write a file pointing to
        // where the asset ended up.

        Ok(())
    }
}

/// Represents a file for which we may or may not have read its contents or
/// calculated their hash.
struct LazyFileContents<'a> {
    path: Cow<'a, Path>,
    contents: Option<Vec<u8>>,
    hash: Option<String>,
}

impl<'a> LazyFileContents<'a> {
    fn new(path: impl Into<Cow<'a, Path>>) -> Self {
        Self {
            path: path.into(),
            contents: None,
            hash: None,
        }
    }

    fn get(&mut self) -> Result<&[u8], SyncError> {
        if self.contents.is_some() {
            Ok(self.contents.as_ref().unwrap())
        } else {
            let contents = fs::read(&self.path).context(error::Io { path: &*self.path })?;

            self.contents = Some(contents);
            Ok(self.contents.as_ref().unwrap())
        }
    }

    fn hash(&mut self) -> Result<&str, SyncError> {
        if self.hash.is_some() {
            Ok(self.hash.as_ref().unwrap())
        } else {
            let contents = self.get()?;
            self.hash = Some(generate_asset_hash(contents));
            Ok(self.hash.as_ref().unwrap())
        }
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
