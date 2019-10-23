mod auth_cookie;
mod commands;
mod config;
mod manifest;
mod options;
mod roblox_web_api;

use std::{error::Error, process};

use structopt::StructOpt;

use crate::options::{Options, Subcommand};

fn main() {
    env_logger::init();

    let options = Options::from_args();

    match run(options) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn run(options: Options) -> Result<(), Box<dyn Error>> {
    match options.command {
        Subcommand::UploadImage(upload_options) => {
            commands::upload_image(options.global, upload_options);
        }
        Subcommand::Sync(sync_options) => {
            commands::sync(options.global, sync_options)?;
        }
    }

    Ok(())
}
