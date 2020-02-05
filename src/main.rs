mod alpha_bleed;
mod asset_name;
mod auth_cookie;
mod codegen;
mod commands;
mod data;
mod dpi_scale;
mod glob;
mod image;
mod options;
mod roblox_web_api;
mod sync_backend;

use std::{error::Error, process};

use structopt::StructOpt;

use crate::options::{Options, Subcommand};

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

fn main() {
    let options = Options::from_args();

    {
        let log_filter = match options.global.verbosity {
            0 => "warn",
            1 => "warn,tarmac=info",
            2 => "warn,tarmac=debug",
            3 => "warn,tarmac=trace",
            _ => "trace",
        };

        let log_env = env_logger::Env::default().default_filter_or(log_filter);

        env_logger::Builder::from_env(log_env)
            .format_timestamp(None)
            .init();
    }

    let panic_result = std::panic::catch_unwind(|| {
        if let Err(err) = run(options) {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    });

    if let Err(error) = panic_result {
        let message = match error.downcast_ref::<&str>() {
            Some(message) => message.to_string(),
            None => match error.downcast_ref::<String>() {
                Some(message) => message.clone(),
                None => "<no message>".to_string(),
            },
        };

        show_crash_message(&message);
        process::exit(2);
    }
}

fn show_crash_message(message: &str) {
    eprintln!(include_str!("crash-message.txt"), message);
}
