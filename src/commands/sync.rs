use std::{
    env, fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use snafu::ResultExt;

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

    let project = ProjectConfig::read_from_folder_or_file(&fuzzy_project_path)
        .context(error::ProjectConfig)?;

    let mut session = SyncSession::new(project)?;

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
    session.codegen()?;

    Ok(())
}

/// A sync session holds all of the state for a single run of the 'tarmac sync'
/// command.
#[derive(Debug)]
struct SyncSession {
    project: ProjectConfig,

    /// The path where this sync session was started from.
    /// $root_path/tarmac-manifest.toml will be updated when the sync session is
    /// over.
    root_path: PathBuf,

    /// The contents of the original manifest from the session's root path, or
    /// the default value if it wasn't present.
    source_manifest: Manifest,
}

impl SyncSession {
    fn new(project: ProjectConfig) -> Result<Self, SyncError> {
        log::trace!("Starting new sync session");

        let root_path = project.file_path.parent().unwrap().to_owned();

        let source_manifest = Manifest::read_from_folder(&root_path)
            .context(error::Manifest)?
            .unwrap_or_default();

        Ok(Self {
            project,
            root_path,
            source_manifest,
        })
    }

    fn sync_to_roblox(&mut self, auth: String) -> Result<(), SyncError> {
        let mut api_client = RobloxApiClient::new(auth);

        // for asset in &mut self.assets {
        //     let asset_content = fs::read(&asset.path).context(error::Io { path: &asset.path })?;
        //     let asset_hash = generate_asset_hash(&asset_content);

        //     let need_to_upload = if asset.manifest_entry.uploaded_id.is_some() {
        //         // If this asset has been uploaded before, compare the content
        //         // hash to see if it's changed since the most recent upload.
        //         match &asset.manifest_entry.uploaded_hash {
        //             Some(existing_hash) => existing_hash != &asset_hash,
        //             None => true,
        //         }
        //     } else {
        //         // If we haven't uploaded this asset before, we definitely need
        //         // to upload it.
        //         true
        //     };

        //     if need_to_upload {
        //         println!("Uploading {}", asset.name);

        //         let uploaded_name = asset.path.file_stem().unwrap().to_str().unwrap();

        //         let response = api_client
        //             .upload_image(ImageUploadData {
        //                 image_data: asset_content,
        //                 name: uploaded_name,
        //                 description: "Uploaded by Tarmac.",
        //             })
        //             .expect("Upload failed");

        //         asset.manifest_entry.uploaded_id = Some(response.backing_asset_id);
        //         asset.manifest_entry.uploaded_hash = Some(asset_hash);
        //     }
        // }

        Ok(())
    }

    fn sync_to_content_folder(&mut self) -> Result<(), SyncError> {
        unimplemented!("TODO: Implement syncing to the content folder");
    }

    fn codegen(&self) -> Result<(), SyncError> {
        // for asset in &self.assets {
        //     log::trace!("Running codegen for {}", asset.name);

        //     match asset.config.codegen {
        //         CodegenKind::None => {}
        //         CodegenKind::AssetUrl => {
        //             if let Some(id) = asset.manifest_entry.uploaded_id {
        //                 let path = &asset.path.with_extension(".lua");
        //                 let contents = format!("return \"rbxassetid://{}\"", id);

        //                 fs::write(path, contents).context(error::Io { path })?;
        //             } else {
        //                 log::warn!(
        //                     "Skipping codegen for asset {} since it was not uploaded.",
        //                     asset.name
        //                 );
        //             }
        //         }
        //         CodegenKind::Slice => {
        //             if let Some(id) = asset.manifest_entry.uploaded_id {
        //                 let path = &asset.path.with_extension(".lua");

        //                 let contents = match &asset.manifest_entry.uploaded_subslice {
        //                     Some(slice) => format!(
        //                         "return {{\
        //                          \n\tImage = \"rbxassetid://{}\",\
        //                          \n\tImageRectOffset = Vector2.new({}, {}),\
        //                          \n\tImageRectSize = Vector2.new({}, {}),\
        //                          }}",
        //                         id, slice.offset.0, slice.offset.1, slice.size.0, slice.size.1,
        //                     ),
        //                     None => format!(
        //                         "return {{\
        //                          \n\tImage = \"rbxassetid://{}\",\
        //                          }}",
        //                         id
        //                     ),
        //                 };

        //                 fs::write(path, contents).context(error::Io { path })?;
        //             } else {
        //                 log::warn!(
        //                     "Skipping codegen for asset {} since it was not uploaded.",
        //                     asset.name
        //                 );
        //             }
        //         }
        //     }
        // }

        Ok(())
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
