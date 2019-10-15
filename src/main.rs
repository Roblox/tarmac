mod auth_cookie;
mod roblox_web_api;

use std::{fs, path::PathBuf};

use structopt::StructOpt;

use crate::{
    auth_cookie::get_auth_cookie,
    roblox_web_api::{ImageUploadData, RobloxApiClient},
};

#[derive(Debug, StructOpt)]
#[structopt(about = "A tool to help manage Roblox assets from the command line")]
struct Options {
    #[structopt(subcommand)]
    command: Subcommand,
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    /// Upload a single image to Roblox.com. Prints the asset ID of the resulting
    /// Image asset to stdout.
    UploadImage(UploadImage),
}

#[derive(Debug, StructOpt)]
struct UploadImage {
    /// The path to the image to upload.
    path: PathBuf,

    /// The name to give to the resulting Decal asset.
    #[structopt(long)]
    name: String,

    /// The description to give to the resulting Decal asset.
    #[structopt(long, default_value = "Uploaded by Tarmac.")]
    description: String,
}

fn main() {
    env_logger::init();

    let options = Options::from_args();

    match &options.command {
        Subcommand::UploadImage(upload_options) => {
            let auth_cookie = get_auth_cookie().expect("no auth cookie");
            let mut client = RobloxApiClient::new(auth_cookie);

            let upload_data = ImageUploadData {
                image_data: fs::read(&upload_options.path).unwrap(),
                name: &upload_options.name,
                description: &upload_options.description,
            };

            let response = client.upload_image(upload_data).expect("request failed");

            println!("{}", response.backing_asset_id);
        }
    }
}
