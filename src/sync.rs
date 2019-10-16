use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::roblox_web_api::{ImageUploadData, RobloxApiClient, UploadResponse};

pub fn sync(path: &Path, auth: String) -> io::Result<()> {
    let mut api_client = RobloxApiClient::new(auth);

    for entry in WalkDir::new(path) {
        let entry = entry?;
        let path = entry.path();

        if is_image_asset(path) {
            sync_image(&mut api_client, path)?;
        }
    }

    Ok(())
}

fn sync_image(api_client: &mut RobloxApiClient, path: &Path) -> io::Result<()> {
    let manifest_path = manifest_path_for_asset(path);
    let manifest = read_manifest(&manifest_path)?;

    let asset_content = fs::read(path)?;
    let asset_hash = generate_asset_hash(&asset_content);

    let manifest = if need_to_upload(manifest.as_ref(), &asset_hash) {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let response = upload_image(api_client, name, asset_content);

        let new_manifest = AssetManifest {
            asset_id: Some(response.backing_asset_id),
            last_uploaded_hash: Some(asset_hash.clone()),
        };

        write_manifest(&manifest_path, &new_manifest)?;

        new_manifest
    } else {
        manifest.unwrap()
    };

    // At this point, the manifest should have an asset ID.
    let asset_id = manifest.asset_id.unwrap();

    let lua_path = lua_path_for_asset(path);
    write_generated_lua(&lua_path, asset_id)?;

    Ok(())
}

fn upload_image(
    api_client: &mut RobloxApiClient,
    name: &str,
    image_data: Vec<u8>,
) -> UploadResponse {
    api_client
        .upload_image(ImageUploadData {
            image_data,
            name,
            description: "Uploaded by Tarmac.",
        })
        .unwrap()
}

fn need_to_upload(maybe_manifest: Option<&AssetManifest>, hash: &str) -> bool {
    match maybe_manifest {
        Some(manifest) => {
            if manifest.asset_id.is_none() {
                return true;
            }

            match &manifest.last_uploaded_hash {
                Some(last_hash) => hash != last_hash,
                None => true,
            }
        }
        None => true,
    }
}

fn is_image_asset(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("png") | Some("jpg") => true,
        _ => false,
    }
}

fn lua_path_for_asset(path: &Path) -> PathBuf {
    path.with_extension("lua")
}

fn write_generated_lua(path: &Path, asset_id: u64) -> io::Result<()> {
    let content = format!("return \"rbxassetid://{}\"", asset_id);
    fs::write(path, &content)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetManifest {
    asset_id: Option<u64>,
    last_uploaded_hash: Option<String>,
}

fn manifest_path_for_asset(path: &Path) -> PathBuf {
    path.with_extension("tarmac.json")
}

fn read_manifest(path: &Path) -> io::Result<Option<AssetManifest>> {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(ref err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };

    Ok(Some(serde_json::from_slice(&content).unwrap()))
}

fn write_manifest(path: &Path, manifest: &AssetManifest) -> io::Result<()> {
    let encoded = serde_json::to_vec(manifest).unwrap();
    fs::write(path, &encoded)
}

fn generate_asset_hash(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}
