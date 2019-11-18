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
    data::{GroupConfig, GroupManifest, InputConfig, InputManifest, Manifest, ProjectConfig},
    options::{GlobalOptions, SyncOptions, SyncTarget},
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

mod error {
    use crate::data::{InputConfigError, ManifestError, ProjectConfigError};
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
        InputConfig {
            source: InputConfigError,
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

    session.gather_inputs()?;

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
    /// The project file pulled from the starting point of the sync operation.
    project: ProjectConfig,

    /// The manifest file that was present as of the beginning of the sync
    /// operation.
    original_manifest: Manifest,

    /// All of the groups and their information in the current sync.
    groups: HashMap<String, SyncGroup>,

    /// All of the inputs discovered so far in the current sync.
    inputs: HashMap<AssetName, SyncInput>,

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

        let groups = project
            .groups
            .iter()
            .map(|(name, config)| {
                (
                    name.clone(),
                    SyncGroup {
                        config: config.clone(),
                        inputs: HashSet::new(),
                    },
                )
            })
            .collect();

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
            groups,
            inputs: HashMap::new(),
        })
    }

    /// Traverse through all known groups and find relevant input files.
    fn gather_inputs(&mut self) -> Result<(), SyncError> {
        let mut paths_to_visit: VecDeque<(InputConfig, PathBuf)> = VecDeque::new();

        for group in self.groups.values_mut() {
            for input_path in &group.config.paths {
                paths_to_visit.push_back((InputConfig::default(), input_path.clone()));

                while let Some((input_config, input_path)) = paths_to_visit.pop_front() {
                    let meta =
                        fs::metadata(&input_path).context(error::Io { path: &input_path })?;

                    if meta.is_file() {
                        if is_image_asset(&input_path) {
                            let asset_name = AssetName::from_paths(&self.root_path, &input_path);

                            self.inputs.insert(
                                asset_name.clone(),
                                SyncInput {
                                    path: input_path,
                                    config: input_config,
                                },
                            );

                            group.inputs.insert(asset_name);
                        }
                    } else {
                        let child_input_config = match InputConfig::read_from_folder(&input_path) {
                            Ok(config) => config,
                            Err(err) if err.is_not_found() => input_config.clone(),
                            other => other.context(error::InputConfig)?,
                        };

                        let children =
                            fs::read_dir(&input_path).context(error::Io { path: &input_path })?;

                        for entry in children {
                            let entry = entry.context(error::Io { path: &input_path })?;
                            paths_to_visit.push_back((child_input_config.clone(), entry.path()));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn sync_to_roblox(&mut self, auth: String) -> Result<(), SyncError> {
        log::info!("Syncing to Roblox...");

        let mut api_client = RobloxApiClient::new(auth);

        for (group_name, group) in &mut self.groups {
            let should_sync_group = {
                // We should sync if any of these are true:
                // - The group's config is different
                // - The set of input assets is different
                // - Any of the inputs' configs are different
                // - Any of the inputs' hashes are different

                // TODO: We always sync every group for now.
                true
            };

            if should_sync_group {
                log::info!("Syncing group {}", group_name);
                log::debug!("Clustering group into clumps");

                // Within groups, we should group together assets that are
                // elgible to be packed together. Assets that can't be packed
                // should be put into their own clump.
                //
                // Only images can be packed. Two image inputs in a group are
                // eligible to be packed if:
                // - They're both marked as eligible for packing
                // - They both have the same DPI scale
                //
                // TODO: For now, we just put every input into its own clump.
                let mut clumps: Vec<Vec<AssetName>> = Vec::new();

                // TODO: Turn this into some smarter clustering algorithm.
                for input_name in &group.inputs {
                    clumps.push(vec![input_name.clone()]);
                }

                log::debug!("Categorized and clumped assets: {:#?}", clumps);

                for clump in clumps {
                    if let [only_member] = clump.as_slice() {
                        let input = self.inputs.get_mut(&only_member).unwrap();

                        let uploaded_name = input.path.file_stem().unwrap().to_str().unwrap();
                        let image_data =
                            fs::read(&input.path).context(error::Io { path: &input.path })?;
                        let hash = generate_asset_hash(&image_data);

                        log::info!("Uploading {}", &only_member);

                        let response = api_client
                            .upload_image(ImageUploadData {
                                image_data,
                                name: uploaded_name,
                                description: "Uploaded by Tarmac.",
                            })
                            .expect("Upload failed");

                        log::info!(
                            "Uploaded {} to ID {}",
                            &only_member,
                            response.backing_asset_id
                        );
                    } else {
                        unimplemented!("Collecting multiple assets in a clump into spritesheets");
                    }
                }
            } else {
                log::info!("Skipping group {}", group_name);
            }
        }

        log::info!("Sync to Roblox done");

        Ok(())
    }

    fn sync_to_content_folder(&mut self) -> Result<(), SyncError> {
        unimplemented!("TODO: Implement syncing to the content folder");
    }

    fn write_manifest(&self) -> Result<(), SyncError> {
        let groups = self
            .groups
            .iter()
            .map(|(name, group)| {
                (
                    name.clone(),
                    GroupManifest {
                        config: group.config.clone(),
                        inputs: group.inputs.iter().cloned().collect(),
                        outputs: BTreeSet::new(),
                    },
                )
            })
            .collect();

        let inputs = self
            .inputs
            .iter()
            .map(|(name, _input)| {
                (
                    name.clone(),
                    InputManifest {
                        uploaded_config: None,
                        uploaded_hash: None,
                        uploaded_id: None,
                        uploaded_slice: None,
                    },
                )
            })
            .collect();

        let manifest = Manifest { groups, inputs };

        manifest
            .write_to_folder(&self.root_path)
            .context(error::Manifest)?;

        Ok(())
    }
}

#[derive(Debug)]
struct SyncInput {
    path: PathBuf,
    config: InputConfig,
}

#[derive(Debug)]
struct SyncGroup {
    config: GroupConfig,
    inputs: HashSet<AssetName>,
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
