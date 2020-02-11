use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};

use packos::{InputItem, SimplePacker};
use snafu::ResultExt;
use walkdir::WalkDir;

use crate::{
    alpha_bleed::alpha_bleed,
    asset_name::AssetName,
    auth_cookie::get_auth_cookie,
    codegen::{AssetUrlTemplate, UrlAndSliceTemplate},
    data::{CodegenKind, Config, ImageSlice, InputManifest, Manifest, SyncInput},
    dpi_scale::dpi_scale_for_path,
    image::Image,
    options::{GlobalOptions, SyncOptions, SyncTarget},
    roblox_web_api::RobloxApiClient,
    sync_backend::{
        ContentSyncBackend, DebugSyncBackend, Error as SyncBackendError, RobloxSyncBackend,
        SyncBackend, UploadInfo,
    },
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
        .map(RobloxApiClient::new);

    let mut session = SyncSession::new(&fuzzy_config_path)?;

    session.discover_configs()?;
    session.discover_inputs()?;

    match options.target {
        SyncTarget::Roblox => {
            let api_client = api_client.as_mut().ok_or(Error::NoAuth)?;
            let mut strategy = RobloxSyncBackend::new(api_client);

            session.sync(&mut strategy)?;
        }
        SyncTarget::ContentFolder => {
            let mut strategy = ContentSyncBackend {};

            session.sync(&mut strategy)?;
        }
        SyncTarget::Debug => {
            let mut strategy = DebugSyncBackend::new();
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
            configs: vec![root_config],
            original_manifest,
            inputs: BTreeMap::new(),
        })
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
    fn discover_configs(&mut self) -> Result<(), Error> {
        let mut to_search = VecDeque::new();
        to_search.extend(
            self.root_config()
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

                self.configs.push(config);
            } else {
                // If this directory contains a config file, we can stop
                // traversing this branch.

                match Config::read_from_folder(&search_path) {
                    Ok(config) => {
                        // We found a config, we're done here.

                        // Append config include paths from this config
                        to_search
                            .extend(config.includes.iter().map(|include| include.path.clone()));

                        self.configs.push(config);
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

                    let name = AssetName::from_paths(config_path, &path);
                    log::trace!("Found input {}", name);

                    let contents = fs::read(&path).context(error::Io { path: &path })?;
                    let hash = generate_asset_hash(&contents);

                    let already_found = inputs.insert(
                        name,
                        SyncInput {
                            path,
                            config: input_config.clone(),
                            contents,
                            hash,
                            id: None,
                            slice: None,
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

    fn sync<S: SyncBackend>(&mut self, backend: &mut S) -> Result<(), Error> {
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
                dpi_scale: dpi_scale_for_path(&input.path),
            };

            let input_group = compatible_input_groups.entry(kind).or_insert_with(Vec::new);

            input_group.push(input_name.clone());
        }

        for (kind, group) in compatible_input_groups {
            if kind.packable {
                self.sync_packable_images(backend, group)?;
            } else {
                for input_name in group {
                    self.sync_unpackable_image(backend, &input_name)?;
                }
            }
        }

        // TODO: Clean up output of inputs that were present in the previous
        // sync but are no longer present.

        Ok(())
    }

    fn sync_packable_images<S: SyncBackend>(
        &mut self,
        backend: &mut S,
        group: Vec<AssetName>,
    ) -> Result<(), SyncError> {
        if self.are_inputs_unchanged(&group) {
            log::info!("Skipping image packing as all inputs are unchanged.");

            for name in &group {
                let input = self.inputs.get_mut(name).unwrap();
                let manifest = &self.original_manifest.inputs[name];

                input.id = manifest.id;
                input.slice = manifest.slice;
            }

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
            let image = Image::decode_png(input.contents.as_slice()).context(error::PngDecode)?;

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
            name: AssetName::spritesheet(),
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
        strategy: &mut S,
        input_name: &AssetName,
    ) -> Result<(), Error> {
        let input = self.inputs.get_mut(input_name).unwrap();

        let upload_data = UploadInfo {
            name: input_name.clone(),
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

                strategy.upload(upload_data)?.id
            } else if let Some(prev_id) = input_manifest.id {
                // The file's contents are the same as the previous sync and
                // this image has been uploaded previously.

                if input_manifest.packable != input.config.packable
                    || input_manifest.codegen != input.config.codegen
                {
                    // Only the file's config has changed.
                    //
                    // TODO: We might not need to reupload this image?

                    log::trace!("Config changed...");

                    strategy.upload(upload_data)?.id
                } else {
                    // Nothing has changed, we're good to go!

                    log::trace!("Input is unchanged");

                    prev_id
                }
            } else {
                // This image has never been uploaded, but its hash is present
                // in the manifest.

                log::trace!("Image has never been uploaded...");

                strategy.upload(upload_data)?.id
            }
        } else {
            // This input was added since the last sync, if there was one.

            log::trace!("Image was added since last sync...");

            strategy.upload(upload_data)?.id
        };

        input.id = Some(id);

        Ok(())
    }

    fn write_manifest(&self) -> Result<(), Error> {
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
                        codegen: input.config.codegen,
                    },
                )
            })
            .collect();

        manifest
            .write_to_folder(self.root_config().folder())
            .context(error::Manifest)?;

        Ok(())
    }

    fn codegen(&self) -> Result<(), Error> {
        log::trace!("Starting codegen");

        for (input_name, input) in &self.inputs {
            log::trace!(
                "Using codegen '{:?}' for {}",
                input.config.codegen,
                input_name
            );

            match input.config.codegen {
                CodegenKind::None => {}

                CodegenKind::AssetUrl => {
                    if let Some(id) = input.id {
                        let template = AssetUrlTemplate { id };

                        let path = &input.path.with_extension("lua");
                        let mut file = File::create(path).context(error::Io { path })?;
                        write!(&mut file, "{}", template).context(error::Io { path })?;

                        log::trace!("Generated code at {}", path.display());
                    } else {
                        log::trace!("Skipping codegen because this input was not uploaded.");
                    }
                }

                CodegenKind::UrlAndSlice => {
                    if let Some(id) = input.id {
                        let template = UrlAndSliceTemplate {
                            id,
                            slice: input.slice,
                        };

                        let path = &input.path.with_extension("lua");
                        let mut file = File::create(path).context(error::Io { path })?;
                        write!(&mut file, "{}", template).context(error::Io { path })?;

                        log::trace!("Generated code at {}", path.display());
                    } else {
                        log::trace!("Skipping codegen because this input was not uploaded.");
                    }
                }
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

mod error {
    use super::*;

    use crate::data::{ConfigError, ManifestError};
    use snafu::Snafu;
    use std::{io, path::PathBuf};

    #[derive(Debug, Snafu)]
    #[snafu(visibility = "pub(super)")]
    pub enum Error {
        #[snafu(display("{}", source))]
        Config {
            source: ConfigError,
        },

        #[snafu(display("{}", source))]
        Backend {
            source: SyncBackendError,
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

        #[snafu(display(
            "Input {} has unsupported png format {:?} (requires RGBA format)",
            path.display(),
            format,
        ))]
        UnsupportedFormat {
            path: PathBuf,
            format: png::ColorType,
        },

        // TODO: Add more detail here and better display
        #[snafu(display("{}", source))]
        PngDecode {
            source: png::DecodingError,
        },

        // TODO: Add more detail here and better display
        #[snafu(display("{}", source))]
        PngEncode {
            source: png::EncodingError,
        },
    }

    impl From<png::EncodingError> for Error {
        fn from(source: png::EncodingError) -> Self {
            Self::PngEncode { source }
        }
    }

    impl From<SyncBackendError> for Error {
        fn from(source: SyncBackendError) -> Self {
            Self::Backend { source }
        }
    }
}
