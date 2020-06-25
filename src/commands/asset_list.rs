use std::collections::BTreeSet;
use std::env;
use std::io::{BufWriter, Write};

use fs_err as fs;

use crate::data::Manifest;
use crate::options::{AssetListOptions, GlobalOptions};

pub fn asset_list(_global: GlobalOptions, options: AssetListOptions) -> anyhow::Result<()> {
    let project_path = match options.project_path {
        Some(path) => path,
        None => env::current_dir()?,
    };

    let manifest = Manifest::read_from_folder(&project_path)?;

    let mut asset_list = BTreeSet::new();
    for input_manifest in manifest.inputs.values() {
        if let Some(id) = input_manifest.id {
            asset_list.insert(id);
        }
    }

    let mut file = BufWriter::new(fs::File::create(&options.output)?);
    for id in asset_list {
        writeln!(file, "{}", id)?;
    }
    file.flush()?;

    Ok(())
}
