use std::collections::BTreeMap;
use std::env;
use std::io::{BufWriter, Write};

use fs_err as fs;

use crate::data::Manifest;
use crate::options::CreatePathMapOptions;

pub fn create_path_map(options: CreatePathMapOptions) -> anyhow::Result<()> {
    let project_path = match options.project_path {
        Some(path) => path.clone(),
        None => env::current_dir()?,
    };

    let manifest = Manifest::read_from_folder(&project_path)?;

    let mut entries: BTreeMap<u64, Vec<_>> = BTreeMap::new();
    for (name, input_manifest) in &manifest.inputs {
        if let Some(id) = input_manifest.id {
            let paths = entries.entry(id).or_default();
            paths.push(name);
        }
    }

    let output_folder = options.output.parent().unwrap();
    fs::create_dir_all(output_folder)?;

    let mut file = BufWriter::new(fs::File::create(&options.output)?);
    serde_json::to_writer_pretty(&mut file, &entries)?;
    file.flush()?;

    Ok(())
}
