mod auth_cookie;
mod commands;
mod config;
mod manifest;
mod options;
mod roblox_web_api;

use structopt::StructOpt;

use crate::options::{Options, Subcommand};

fn main() {
    env_logger::init();

    let options = Options::from_args();

    match options.command {
        Subcommand::UploadImage(upload_options) => {
            commands::upload_image(options.global, upload_options);
        }
        Subcommand::Sync(sync_options) => {
            commands::sync(options.global, sync_options).unwrap();
        }
    }
}
