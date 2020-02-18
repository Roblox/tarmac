use std::{borrow::Cow, io, path::Path};

use snafu::Snafu;

use crate::{
    asset_name::AssetName,
    fs,
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

pub trait SyncBackend {
    fn upload(&mut self, data: UploadInfo) -> Result<UploadResponse, Error>;
}

pub struct UploadResponse {
    pub id: u64,
}

pub struct UploadInfo {
    pub name: AssetName,
    pub contents: Vec<u8>,
    pub hash: String,
}

pub struct RobloxSyncBackend<'a> {
    api_client: &'a mut RobloxApiClient,
}

impl<'a> RobloxSyncBackend<'a> {
    pub fn new(api_client: &'a mut RobloxApiClient) -> Self {
        Self { api_client }
    }
}

impl<'a> SyncBackend for RobloxSyncBackend<'a> {
    fn upload(&mut self, data: UploadInfo) -> Result<UploadResponse, Error> {
        log::info!("Uploading {} to Roblox", &data.name);

        let response = self.api_client.upload_image(ImageUploadData {
            image_data: Cow::Owned(data.contents),
            name: data.name.as_ref(),
            description: "Uploaded by Tarmac.",
        })?;

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

pub struct ContentSyncBackend {
    // TODO: Studio install information
}

impl SyncBackend for ContentSyncBackend {
    fn upload(&mut self, _data: UploadInfo) -> Result<UploadResponse, Error> {
        unimplemented!("content folder uploading");
    }
}

pub struct DebugSyncBackend {
    last_id: u64,
}

impl DebugSyncBackend {
    pub fn new() -> Self {
        Self { last_id: 0 }
    }
}

impl SyncBackend for DebugSyncBackend {
    fn upload(&mut self, data: UploadInfo) -> Result<UploadResponse, Error> {
        log::info!("Copying {} to local folder", &data.name);

        self.last_id += 1;
        let id = self.last_id;

        let path = Path::new(".tarmac-debug");
        fs::create_dir_all(path)?;

        let mut file_path = path.join(id.to_string());

        if let Some(ext) = Path::new(data.name.as_ref()).extension() {
            file_path.set_extension(ext);
        }

        fs::write(&file_path, &data.contents)?;

        Ok(UploadResponse { id })
    }
}

#[derive(Debug, Snafu)]
pub enum Error {
    Io { source: io::Error },

    Http { source: reqwest::Error },
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::Io { source }
    }
}

impl From<reqwest::Error> for Error {
    fn from(source: reqwest::Error) -> Self {
        Self::Http { source }
    }
}
