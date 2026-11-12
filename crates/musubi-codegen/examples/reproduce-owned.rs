//! Reproduce a retained SDK into an isolated empty directory using its recorded producer.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use musubi_codegen::{build_ir, load_inputs, read_owned_tree, render};
use musubi_core::GeneratorInfo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 2 {
        return Err("expected a manifest and an existing empty reproduction directory".into());
    }
    let manifest = PathBuf::from(&arguments[0]).canonicalize()?;
    let output = PathBuf::from(&arguments[1]);
    let metadata = std::fs::symlink_metadata(&output)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || output.read_dir()?.next().is_some()
    {
        return Err("reproduction output must be an existing empty ordinary directory".into());
    }
    let output = output.canonicalize()?;
    let inputs = load_inputs(&manifest)?;
    let owned = manifest
        .parent()
        .ok_or("manifest parent is missing")?
        .join(inputs.output.relative().as_path());
    let retained = read_owned_tree(&owned, inputs.limits)?;
    let release = retained
        .file_entries()
        .find_map(|(path, bytes)| (path == Path::new("musubi-release.json")).then_some(bytes))
        .ok_or("owned SDK release metadata is missing")?;
    let release: serde_json::Value = serde_json::from_slice(release)?;
    let producer: GeneratorInfo = serde_json::from_value(release["generator"].clone())?;
    producer.validate()?;
    let tree = render(&build_ir(inputs)?, &producer)?;
    for file in tree.into_files() {
        let path = output.join(file.path);
        std::fs::create_dir_all(path.parent().ok_or("output parent is missing")?)?;
        std::fs::write(path, file.contents)?;
    }
    Ok(())
}
