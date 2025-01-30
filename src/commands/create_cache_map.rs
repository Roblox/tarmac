use std::collections::BTreeMap;
use std::env;
use std::io::{BufWriter, Write};

use fs_err as fs;

use crate::asset_name::AssetName;
use crate::data::Manifest;
use crate::options::{CreateCacheMapOptions, GlobalOptions};
use crate::roblox_web_api::{RobloxApiClient, RobloxOpenCloudCredentials};

pub fn create_cache_map(
    global: GlobalOptions,
    options: CreateCacheMapOptions,
) -> anyhow::Result<()> {
    let credentials = RobloxOpenCloudCredentials::get_credentials(global.auth, global.api_key)?;
    let mut api_client = RobloxApiClient::new(credentials);

    let project_path = match options.project_path {
        Some(path) => path.clone(),
        None => env::current_dir()?,
    };

    let manifest = Manifest::read_from_folder(&project_path)?;

    let index_dir = options.index_file.parent().unwrap();
    fs::create_dir_all(index_dir)?;

    fs::create_dir_all(&options.cache_dir)?;

    let mut uploaded_inputs: BTreeMap<u64, Vec<&AssetName>> = BTreeMap::new();
    for (name, input_manifest) in &manifest.inputs {
        if let Some(id) = input_manifest.id {
            let paths = uploaded_inputs.entry(id).or_default();
            paths.push(name);
        }
    }

    let mut index: BTreeMap<u64, String> = BTreeMap::new();
    for (id, contributing_assets) in uploaded_inputs {
        if contributing_assets.len() == 1 {
            index.insert(id, contributing_assets[0].to_string());
        } else {
            let contents = api_client.download_image(id)?;
            let path = options.cache_dir.join(id.to_string());
            fs::write(&path, contents)?;

            index.insert(id, path.display().to_string());
        }
    }

    let mut file = BufWriter::new(fs::File::create(&options.index_file)?);
    serde_json::to_writer_pretty(&mut file, &index)?;
    file.flush()?;

    Ok(())
}
