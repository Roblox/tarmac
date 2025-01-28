use std::{path::PathBuf, str::FromStr};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = env!("CARGO_PKG_DESCRIPTION"))]
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

    /// The Open Cloud API key tarmac will use to upload assets.
    #[structopt(long, global(true))]
    pub api_key: Option<String>,

    /// Sets verbosity level. Can be specified multiple times.
    #[structopt(long = "verbose", short, global(true), parse(from_occurrences))]
    pub verbosity: u8,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// Upload a single image to the Roblox cloud. Prints the asset ID of the
    /// resulting Image asset to stdout.
    UploadImage(UploadImageOptions),

    /// Sync your Tarmac project, uploading any assets that have changed.
    Sync(SyncOptions),

    /// Downloads any packed spritesheets, then generates a file mapping asset
    /// IDs to file paths.
    CreateCacheMap(CreateCacheMapOptions),

    /// Creates a file that lists all assets required by the project.
    AssetList(AssetListOptions),
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

    /// The ID of the user to upload to. Not compatible with `group_id`.
    #[structopt(long)]
    pub group_id: Option<u64>,

    /// The ID of the group to upload to. Not compatible with `user_id`.
    #[structopt(long)]
    pub user_id: Option<u64>,
}

#[derive(Debug, StructOpt)]
pub struct SyncOptions {
    /// Where Tarmac should sync the project.
    ///
    /// Options:
    ///
    /// - roblox: Upload to Roblox.com
    ///
    /// - none: Do not upload. Tarmac will exit with an error if there are any
    ///   unsynced assets.
    ///
    /// - debug: Copy to local debug directory for debugging output
    #[structopt(long)]
    pub target: SyncTarget,

    /// When provided, Tarmac will upload again at most the given number of times
    /// when it encounters rate limitation errors.
    #[structopt(long)]
    pub retry: Option<usize>,

    /// The number of seconds to wait between each re-upload attempts.
    #[structopt(long, default_value = "60")]
    pub retry_delay: u64,

    /// The path to a Tarmac config, or a folder containing a Tarmac project.
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncTarget {
    Roblox,
    None,
    Debug,
}

impl FromStr for SyncTarget {
    type Err = String;

    fn from_str(value: &str) -> Result<SyncTarget, Self::Err> {
        match value {
            "roblox" => Ok(SyncTarget::Roblox),
            "none" => Ok(SyncTarget::None),
            "debug" => Ok(SyncTarget::Debug),

            _ => Err(String::from(
                "Invalid sync target. Valid options are roblox, none, and debug.",
            )),
        }
    }
}

#[derive(Debug, StructOpt)]
pub struct CreateCacheMapOptions {
    pub project_path: Option<PathBuf>,

    /// A path to a directory to put any downloaded packed images.
    #[structopt(long = "cache-dir")]
    pub cache_dir: PathBuf,

    /// A path to a file to contain the cache mapping.
    #[structopt(long = "index-file")]
    pub index_file: PathBuf,
}

#[derive(Debug, StructOpt)]
pub struct AssetListOptions {
    pub project_path: Option<PathBuf>,

    /// A path to a file to put the asset list.
    #[structopt(long = "output")]
    pub output: PathBuf,
}
