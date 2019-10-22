mod auth_cookie;
mod config;
mod manifest;
mod options;
mod roblox_web_api;
mod sync;

use std::{env, fs};

use structopt::StructOpt;

use crate::{
    auth_cookie::get_auth_cookie,
    options::{Options, Subcommand},
    roblox_web_api::{ImageUploadData, RobloxApiClient},
    sync::sync,
};

fn main() {
    env_logger::init();

    let options = Options::from_args();

    match &options.command {
        Subcommand::UploadImage(upload_options) => {
            let auth = options
                .global
                .auth
                .clone()
                .or_else(get_auth_cookie)
                .expect("no auth cookie found");

            let image_data = fs::read(&upload_options.path).expect("couldn't read input file");

            let mut client = RobloxApiClient::new(auth);

            let upload_data = ImageUploadData {
                image_data,
                name: &upload_options.name,
                description: &upload_options.description,
            };

            let response = client
                .upload_image(upload_data)
                .expect("Roblox API request failed");

            eprintln!("Image uploaded successfully!");
            println!("{}", response.backing_asset_id);
        }
        Subcommand::Sync(sync_options) => {
            let paths = if sync_options.paths.is_empty() {
                vec![env::current_dir().unwrap()]
            } else {
                sync_options.paths.clone()
            };

            let auth = options
                .global
                .auth
                .clone()
                .or_else(get_auth_cookie)
                .expect("no auth cookie found");

            sync(&paths, auth).unwrap();
        }
    }
}
