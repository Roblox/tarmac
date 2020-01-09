use std::{borrow::Cow, fs};

use crate::{
    auth_cookie::get_auth_cookie,
    options::{GlobalOptions, UploadImageOptions},
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

pub fn upload_image(global: GlobalOptions, options: UploadImageOptions) {
    let auth = global
        .auth
        .clone()
        .or_else(get_auth_cookie)
        .expect("no auth cookie found");

    let image_data = fs::read(options.path).expect("couldn't read input file");

    let mut client = RobloxApiClient::new(auth);

    let upload_data = ImageUploadData {
        image_data: Cow::Owned(image_data),
        name: &options.name,
        description: &options.description,
    };

    let response = client
        .upload_image(upload_data)
        .expect("Roblox API request failed");

    eprintln!("Image uploaded successfully!");
    println!("{}", response.backing_asset_id);
}
