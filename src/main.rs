mod auth_cookie;
mod roblox_web_api;
mod sync;

use std::{env, fs, path::PathBuf};

use structopt::StructOpt;

use crate::{
    auth_cookie::get_auth_cookie,
    roblox_web_api::{ImageUploadData, RobloxApiClient},
    sync::sync,
};

#[derive(Debug, StructOpt)]
#[structopt(about = "A tool to help manage Roblox assets from the command line")]
struct Options {
    /// The authentication cookie for Tarmac to use. If not specified, Tarmac
    /// will attempt to use the cookie from the Roblox Studio installation on
    /// the system.
    #[structopt(long)]
    auth: Option<String>,

    #[structopt(subcommand)]
    command: Subcommand,
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    /// Upload a single image to Roblox.com. Prints the asset ID of the
    /// resulting Image asset to stdout.
    UploadImage(UploadImage),

    /// Sync your Tarmac asset project up to Roblox.com, uploading any assets
    /// that have changed.
    Sync(Sync),
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

#[derive(Debug, StructOpt)]
struct Sync {
    /// The path to the assets to be synced with Roblox.com. Defaults to the
    /// current working directory.
    path: Option<PathBuf>,
}

fn main() {
    env_logger::init();

    let options = Options::from_args();

    match &options.command {
        Subcommand::UploadImage(upload_options) => {
            let auth = options
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
            let path = sync_options
                .path
                .clone()
                .unwrap_or_else(|| env::current_dir().unwrap());

            let auth = options
                .auth
                .clone()
                .or_else(get_auth_cookie)
                .expect("no auth cookie found");

            sync(&path, auth).unwrap();
        }
    }
}
