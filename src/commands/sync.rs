use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    env,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use fs_err as fs;
use packos::{InputItem, SimplePacker};
use thiserror::Error;
use walkdir::WalkDir;

use crate::{
    alpha_bleed::alpha_bleed,
    asset_name::AssetName,
    auth_cookie::get_auth_cookie,
    codegen::perform_codegen,
    data::{Config, ConfigError, ImageSlice, InputManifest, Manifest, ManifestError, SyncInput},
    dpi_scale,
    image::Image,
    options::{GlobalOptions, SyncOptions, SyncTarget},
    roblox_web_api::{RobloxApiClient, RobloxApiError},
    sync_backend::{
        DebugSyncBackend, Error as SyncBackendError, NoneSyncBackend, RetryBackend,
        RobloxSyncBackend, SyncBackend, UploadInfo,
    },
};

fn sync_session<B: SyncBackend>(session: &mut SyncSession, options: &SyncOptions, mut backend: B) {
    if let Some(retry) = options.retry {
        let mut retry_backend =
            RetryBackend::new(backend, retry, Duration::from_secs(options.retry_delay));
        session.sync_with_backend(&mut retry_backend);
    } else {
        session.sync_with_backend(&mut backend);
    }
}

pub fn sync(global: GlobalOptions, options: SyncOptions) -> Result<(), SyncError> {
    let fuzzy_config_path = match &options.config_path {
        Some(v) => v.to_owned(),
        None => env::current_dir()?,
    };

    let mut api_client = RobloxApiClient::new(global.auth.or_else(get_auth_cookie));

    let mut session = SyncSession::new(&fuzzy_config_path)?;

    session.discover_configs()?;
    session.discover_inputs()?;

    match &options.target {
        SyncTarget::Roblox => {
            let group_id = session.root_config().upload_to_group_id;
            sync_session(
                &mut session,
                &options,
                RobloxSyncBackend::new(&mut api_client, group_id),
            );
        }
        SyncTarget::None => {
            sync_session(&mut session, &options, NoneSyncBackend);
        }
        SyncTarget::Debug => {
            sync_session(&mut session, &options, DebugSyncBackend::new());
        }
    }

    session.write_manifest()?;
    session.codegen()?;
    session.write_asset_list()?;
    session.populate_asset_cache(&mut api_client)?;

    if session.sync_errors.is_empty() {
        Ok(())
    } else {
        Err(SyncError::HadErrors {
            error_count: session.sync_errors.len(),
        })
    }
}

/// A sync session holds all of the state for a single run of the 'tarmac sync'
/// command.
#[derive(Debug)]
struct SyncSession {
    /// The set of all configs known by the sync session.
    ///
    /// This list is always at least one element long. The first entry is the
    /// root config where the sync session was started; use
    /// SyncSession::root_config to retrieve it.
    configs: Vec<Config>,

    /// The manifest file that was present as of the beginning of the sync
    /// operation.
    original_manifest: Manifest,

    /// All of the inputs discovered so far in the current sync.
    inputs: BTreeMap<AssetName, SyncInput>,

    /// Errors encountered during syncing that we ignored at the time.
    sync_errors: Vec<anyhow::Error>,
}

/// Contains information to help Tarmac batch process different kinds of assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InputKind {
    packable: bool,
    dpi_scale: u32,
}

struct PackedImage {
    image: Image,
    slices: HashMap<AssetName, ImageSlice>,
}

impl SyncSession {
    fn new(fuzzy_config_path: &Path) -> Result<Self, SyncError> {
        log::trace!("Starting new sync session");

        let root_config = Config::read_from_folder_or_file(&fuzzy_config_path)?;

        log::trace!("Starting from config \"{}\"", root_config.name);

        let original_manifest = match Manifest::read_from_folder(root_config.folder()) {
            Ok(manifest) => manifest,
            Err(err) if err.is_not_found() => Manifest::default(),
            other => other?,
        };

        Ok(Self {
            configs: vec![root_config],
            original_manifest,
            inputs: BTreeMap::new(),
            sync_errors: Vec::new(),
        })
    }

    /// Raise a sync error that will fail the sync process at a later point.
    fn raise_error(&mut self, error: impl Into<anyhow::Error>) {
        let error = error.into();
        log::error!("{:?}", error);
        self.sync_errors.push(error);
    }

    /// The config that this sync session was started from.
    fn root_config(&self) -> &Config {
        &self.configs[0]
    }

    /// Locate all of the configs connected to our root config.
    ///
    /// Tarmac config files can include each other via the `includes` field,
    /// which will search the given path for other config files and use them as
    /// part of the sync.
    fn discover_configs(&mut self) -> Result<(), SyncError> {
        let mut to_search = VecDeque::new();
        to_search.extend(self.root_config().includes.iter().cloned());

        while let Some(search_path) = to_search.pop_front() {
            let search_meta = fs::metadata(&search_path)?;

            if search_meta.is_file() {
                // This is a file that's explicitly named by a config. We'll
                // check that it's a Tarmac config and include it.

                let config = Config::read_from_file(&search_path)?;

                // Include any configs that this config references.
                to_search.extend(config.includes.iter().cloned());

                self.configs.push(config);
            } else {
                // If this directory contains a config file, we can stop
                // traversing this branch.

                match Config::read_from_folder(&search_path) {
                    Ok(config) => {
                        // We found a config, we're done here.

                        // Append config include paths from this config
                        to_search.extend(config.includes.iter().cloned());

                        self.configs.push(config);
                    }

                    Err(err) if err.is_not_found() => {
                        // We didn't find a config, keep searching down this
                        // branch of the filesystem.

                        let children = fs::read_dir(&search_path)?;

                        for entry in children {
                            let entry = entry?;
                            let entry_path = entry.path();

                            // DirEntry has a metadata method, but in the case
                            // of symlinks, it returns metadata about the
                            // symlink and not the file or folder.
                            let entry_meta = fs::metadata(&entry_path)?;

                            if entry_meta.is_dir() {
                                to_search.push_back(entry_path);
                            }
                        }
                    }

                    Err(err) => {
                        return Err(err.into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Find all files on the filesystem referenced as inputs by our configs.
    fn discover_inputs(&mut self) -> Result<(), SyncError> {
        let inputs = &mut self.inputs;
        let root_config_path = &self.configs[0].folder();

        // Starting with our root config, iterate over all configs and find all
        // relevant inputs
        for config in &self.configs {
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
                    let path = matching.into_path();

                    let name = AssetName::from_paths(&root_config_path, &path);
                    log::trace!("Found input {}", name);

                    let path_info = dpi_scale::extract_path_info(&path);

                    let contents = fs::read(&path)?;
                    let hash = generate_asset_hash(&contents);

                    // If this input was known during the last sync operation,
                    // pull the information we knew about it out.
                    let (id, slice) = match self.original_manifest.inputs.get(&name) {
                        Some(original) => (original.id, original.slice),
                        None => (None, None),
                    };

                    let already_found = inputs.insert(
                        name.clone(),
                        SyncInput {
                            name,
                            path,
                            path_without_dpi_scale: path_info.path_without_dpi_scale,
                            dpi_scale: path_info.dpi_scale,
                            config: input_config.clone(),
                            contents,
                            hash,
                            id,
                            slice,
                        },
                    );

                    if let Some(existing) = already_found {
                        return Err(SyncError::OverlappingGlobs {
                            path: existing.path,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn sync_with_backend<S: SyncBackend>(&mut self, backend: &mut S) {
        let mut compatible_input_groups = BTreeMap::new();

        for (input_name, input) in &self.inputs {
            if !is_image_asset(&input.path) {
                log::warn!(
                    "Asset '{}' is not recognized by Tarmac.",
                    input.path.display()
                );

                continue;
            }

            let kind = InputKind {
                packable: input.config.packable,
                dpi_scale: input.dpi_scale,
            };

            let input_group = compatible_input_groups.entry(kind).or_insert_with(Vec::new);

            input_group.push(input_name.clone());
        }

        'outer: for (kind, group) in compatible_input_groups {
            if kind.packable {
                if let Err(err) = self.sync_packable_images(backend, group) {
                    let rate_limited = err.is_rate_limited();

                    println!("{}: {:#?}", rate_limited, err);

                    self.raise_error(err);

                    if rate_limited {
                        break 'outer;
                    }
                }
            } else {
                for input_name in group {
                    if let Err(err) = self.sync_unpackable_image(backend, &input_name) {
                        let rate_limited = err.is_rate_limited();

                        self.raise_error(err);

                        if rate_limited {
                            break 'outer;
                        }
                    }
                }
            }
        }

        // TODO: Clean up output of inputs that were present in the previous
        // sync but are no longer present.
    }

    fn sync_packable_images<S: SyncBackend>(
        &mut self,
        backend: &mut S,
        group: Vec<AssetName>,
    ) -> Result<(), SyncError> {
        if self.are_inputs_unchanged(&group) {
            log::info!("Skipping image packing as all inputs are unchanged.");

            return Ok(());
        }

        log::trace!("Packing images...");
        let mut packed_images = self.pack_images(&group)?;

        log::trace!("Alpha-bleeding {} packed images...", packed_images.len());

        for (i, packed_image) in packed_images.iter_mut().enumerate() {
            log::trace!("Bleeding image {}", i);

            alpha_bleed(&mut packed_image.image);
        }

        log::trace!("Syncing packed images...");
        for packed_image in &packed_images {
            self.sync_packed_image(backend, packed_image)?;
        }

        Ok(())
    }

    fn are_inputs_unchanged(&self, group: &[AssetName]) -> bool {
        for name in group {
            if let Some(manifest) = self.original_manifest.inputs.get(name) {
                let input = &self.inputs[name];
                let unchanged = input.is_unchanged_since_last_sync(manifest);

                if !unchanged {
                    log::trace!("Input {} changed since last sync", name);

                    return false;
                }
            } else {
                log::trace!(
                    "Input {} was not present last sync, need to re-pack spritesheets",
                    name
                );

                return false;
            }
        }

        true
    }

    fn pack_images(&self, group: &[AssetName]) -> Result<Vec<PackedImage>, SyncError> {
        let mut packos_inputs = Vec::new();
        let mut images_by_id = HashMap::new();

        for name in group {
            let input = &self.inputs[&name];
            let image = Image::decode_png(input.contents.as_slice())?;

            let input = InputItem::new(image.size());

            images_by_id.insert(input.id(), (name, image));
            packos_inputs.push(input);
        }

        let packer = SimplePacker::new()
            .max_size(self.root_config().max_spritesheet_size)
            .padding(1);

        let pack_results = packer.pack(packos_inputs);
        let mut packed_images = Vec::new();

        for bucket in pack_results.buckets() {
            let mut image = Image::new_empty_rgba8(bucket.size());
            let mut slices: HashMap<AssetName, _> = HashMap::new();

            for item in bucket.items() {
                let (name, sprite_image) = &images_by_id[&item.id()];

                image.blit(sprite_image, item.position());

                let slice = ImageSlice::new(item.position(), item.max());
                slices.insert((*name).clone(), slice);
            }

            packed_images.push(PackedImage { image, slices });
        }

        Ok(packed_images)
    }

    fn sync_packed_image<S: SyncBackend>(
        &mut self,
        backend: &mut S,
        packed_image: &PackedImage,
    ) -> Result<(), SyncError> {
        let mut encoded_image = Vec::new();
        packed_image.image.encode_png(&mut encoded_image)?;

        let hash = generate_asset_hash(&encoded_image);

        let upload_data = UploadInfo {
            name: "spritesheet".to_owned(),
            contents: encoded_image,
            hash: hash.clone(),
        };

        let id = backend.upload(upload_data)?.id;

        // Apply resolved metadata back to the inputs
        for (asset_name, slice) in &packed_image.slices {
            let input = self.inputs.get_mut(asset_name).unwrap();

            input.id = Some(id);
            input.slice = Some(*slice);
        }

        Ok(())
    }

    fn sync_unpackable_image<S: SyncBackend>(
        &mut self,
        backend: &mut S,
        input_name: &AssetName,
    ) -> Result<(), SyncError> {
        let input = self.inputs.get_mut(input_name).unwrap();

        let upload_data = UploadInfo {
            name: input.human_name(),
            contents: input.contents.clone(),
            hash: input.hash.clone(),
        };

        let id = if let Some(input_manifest) = self.original_manifest.inputs.get(&input_name) {
            // This input existed during our last sync operation. We'll compare
            // the current state with the previous one to see if we need to take
            // action.

            if input_manifest.hash != input.hash {
                // The file's contents have been edited since the last sync.

                log::trace!("Contents changed...");

                backend.upload(upload_data)?.id
            } else if input.id.is_some() {
                // The file's contents are the same as the previous sync and
                // this image has been uploaded previously.

                if input_manifest.packable != input.config.packable {
                    // Only the file's config has changed.
                    //
                    // TODO: We might not need to reupload this image?

                    log::trace!("Config changed...");

                    backend.upload(upload_data)?.id
                } else {
                    // Nothing has changed, we're good to go!

                    log::trace!("Input is unchanged.");
                    return Ok(());
                }
            } else {
                // This image has never been uploaded, but its hash is present
                // in the manifest.

                log::trace!("Image has never been uploaded...");

                backend.upload(upload_data)?.id
            }
        } else {
            // This input was added since the last sync, if there was one.

            log::trace!("Image was added since last sync...");

            backend.upload(upload_data)?.id
        };

        input.id = Some(id);

        Ok(())
    }

    fn write_manifest(&self) -> Result<(), SyncError> {
        log::trace!("Generating new manifest");

        let mut manifest = Manifest::default();

        manifest.inputs = self
            .inputs
            .iter()
            .map(|(name, input)| {
                (
                    name.clone(),
                    InputManifest {
                        hash: input.hash.clone(),
                        id: input.id,
                        slice: input.slice,
                        packable: input.config.packable,
                    },
                )
            })
            .collect();

        manifest.write_to_folder(self.root_config().folder())?;

        Ok(())
    }

    fn codegen(&self) -> Result<(), SyncError> {
        log::trace!("Starting codegen");

        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct CodegenCompatibility<'a> {
            output_path: Option<&'a Path>,
        }

        let mut compatible_codegen_groups = HashMap::new();

        for (input_name, input) in &self.inputs {
            let output_path = input
                .config
                .codegen_path
                .as_ref()
                .map(|path| path.as_path());

            let compat = CodegenCompatibility { output_path };

            let group = compatible_codegen_groups
                .entry(compat)
                .or_insert_with(Vec::new);
            group.push(input_name.clone());
        }

        for (compat, names) in compatible_codegen_groups {
            let inputs: Vec<_> = names.iter().map(|name| &self.inputs[name]).collect();
            let output_path = compat.output_path;

            perform_codegen(output_path, &inputs)?;
        }

        Ok(())
    }

    fn write_asset_list(&self) -> Result<(), SyncError> {
        let list_path = match &self.root_config().asset_list_path {
            Some(path) => path,
            None => return Ok(()),
        };

        log::debug!("Writing asset list");

        let list_parent = list_path.parent().unwrap();
        fs_err::create_dir_all(list_parent)?;

        let mut file = BufWriter::new(fs_err::File::create(list_path)?);

        let known_ids: BTreeSet<u64> = self.inputs.values().filter_map(|input| input.id).collect();

        for id in known_ids {
            writeln!(file, "rbxassetid://{}", id)?;
        }

        file.flush()?;
        Ok(())
    }

    fn populate_asset_cache(&self, api_client: &mut RobloxApiClient) -> Result<(), SyncError> {
        let cache_path = match &self.root_config().asset_cache_path {
            Some(path) => path,
            None => return Ok(()),
        };

        log::debug!("Populating asset cache");

        fs_err::create_dir_all(&cache_path)?;

        let known_ids: HashSet<u64> = self.inputs.values().filter_map(|input| input.id).collect();

        // Clean up cache items that aren't present in our current project.
        for entry in fs_err::read_dir(&cache_path)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs_err::metadata(&path)?;

            let name_as_id: Option<u64> = path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse().ok());

            let should_clean_up;
            if metadata.is_dir() {
                // Tarmac never generates directories, so we should clean this.
                should_clean_up = true;
            } else if let Some(id) = name_as_id {
                // This file looks like an ID. If it's not present in this
                // project, we assume it's from an old sync and clean it up.
                should_clean_up = !known_ids.contains(&id);
            } else {
                // This is some other file that we should clean up.
                should_clean_up = true;
            }

            if should_clean_up {
                if metadata.is_dir() {
                    fs_err::remove_dir_all(&path)?;
                } else {
                    fs_err::remove_file(&path)?;
                }
            }
        }

        for input in self.inputs.values() {
            if let Some(id) = input.id {
                let input_path = cache_path.join(format!("{}", id));

                match fs_err::metadata(&input_path) {
                    Ok(_) => {
                        // This asset is already downloaded, we can skip it.
                        continue;
                    }
                    Err(err) => {
                        if err.kind() != io::ErrorKind::NotFound {
                            return Err(err.into());
                        }
                    }
                }

                log::debug!("Downloading asset ID {}", id);

                let contents = api_client.download_image(id)?;
                fs_err::write(input_path, contents)?;
            }
        }

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
    format!("{}", blake3::hash(content).to_hex())
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Path {} was described by more than one glob", .path.display())]
    OverlappingGlobs { path: PathBuf },

    #[error("'tarmac sync' completed, but with {error_count} error(s)")]
    HadErrors { error_count: usize },

    #[error(transparent)]
    WalkDir {
        #[from]
        source: walkdir::Error,
    },

    #[error(transparent)]
    Config {
        #[from]
        source: ConfigError,
    },

    #[error(transparent)]
    Backend {
        #[from]
        source: SyncBackendError,
    },

    #[error(transparent)]
    Manifest {
        #[from]
        source: ManifestError,
    },

    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },

    #[error(transparent)]
    PngDecode {
        #[from]
        source: png::DecodingError,
    },

    #[error(transparent)]
    PngEncode {
        #[from]
        source: png::EncodingError,
    },

    #[error(transparent)]
    RobloxApi {
        #[from]
        source: RobloxApiError,
    },
}

impl SyncError {
    pub fn is_rate_limited(&self) -> bool {
        match self {
            Self::Backend {
                source: SyncBackendError::RateLimited { .. },
            } => true,
            _ => false,
        }
    }
}
