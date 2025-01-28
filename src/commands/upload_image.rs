use fs_err as fs;

use crate::{
    options::{GlobalOptions, UploadImageOptions},
    roblox_web_api::{RobloxApiClient, RobloxOpenCloudCredentials, DECAL},
    roblox_web_api_types::{ImageUploadData, ImageUploadMetadata},
};

pub fn upload_image(
    global: GlobalOptions,
    options: UploadImageOptions,
) -> Result<(), anyhow::Error> {
    let image_data = fs::read(options.path).expect("couldn't read input file");
    let credentials = RobloxOpenCloudCredentials::get_credentials(global.auth, global.api_key)?;

    let mut client = RobloxApiClient::new(credentials);

    let upload_data = ImageUploadData {
        image_data: image_data.into(),
        image_metadata: ImageUploadMetadata::new(
            DECAL.to_string(),
            options.name.to_string(),
            options.description.to_string(),
            options.user_id,
            options.group_id,
        )?,
    };

    let response = client.upload_image(upload_data)?;

    eprintln!("Image uploaded successfully!");
    println!("{}", response.asset_id);
    Ok(())
}
