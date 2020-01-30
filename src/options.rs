use std::{path::PathBuf, str::FromStr};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "A tool to help manage Roblox assets from the command line")]
pub struct Options {
    #[structopt(flatten)]
    pub global: GlobalOptions,

    #[structopt(subcommand)]
    pub command: Subcommand,
}

#[derive(Debug, StructOpt)]
pub struct GlobalOptions {
    /// The authentication cookie for Tarmac to use. If not specified, Tarmac
    /// will attempt to use the cookie from the Roblox Studio installation on
    /// the system.
    #[structopt(long, global(true))]
    pub auth: Option<String>,

    /// Sets verbosity level. Can be specified multiple times.
    #[structopt(long = "verbose", short, global(true), parse(from_occurrences))]
    pub verbosity: u8,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// Upload a single image to Roblox.com. Prints the asset ID of the
    /// resulting Image asset to stdout.
    UploadImage(UploadImageOptions),

    /// Sync your Tarmac asset project up to Roblox.com, uploading any assets
    /// that have changed.
    Sync(SyncOptions),
}

#[derive(Debug, StructOpt)]
pub struct UploadImageOptions {
    /// The path to the image to upload.
    pub path: PathBuf,

    /// The name to give to the resulting Decal asset.
    #[structopt(long)]
    pub name: String,

    /// The description to give to the resulting Decal asset.
    #[structopt(long, default_value = "Uploaded by Tarmac.")]
    pub description: String,
}

#[derive(Debug, StructOpt)]
pub struct SyncOptions {
    /// Where Tarmac should put resulting artifacts. This impacts code
    /// generation and what side effects Tarmac performs.
    ///
    /// Options:
    ///
    /// - roblox: Upload to Roblox.com
    /// - content-folder: Copy to content folder with hashed names
    /// - debug: Copy to local debug directory for debugging output
    #[structopt(long)]
    pub target: SyncTarget,

    /// The path to a Tarmac config, or a folder containing a Tarmac config, to
    /// define how to turn files on disk into assets.
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncTarget {
    Roblox,
    ContentFolder,
    Debug,
}

impl FromStr for SyncTarget {
    type Err = String;

    fn from_str(value: &str) -> Result<SyncTarget, Self::Err> {
        match value {
            "roblox" => Ok(SyncTarget::Roblox),
            "content-folder" => Ok(SyncTarget::ContentFolder),
            "debug" => Ok(SyncTarget::Debug),

            _ => Err(String::from(
                "Invalid sync target. Valid options are roblox, content-folder, and debug.",
            )),
        }
    }
}
