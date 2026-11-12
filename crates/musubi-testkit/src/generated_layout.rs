//! Binds semantic owner files and facade module declarations to one closed release authority.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use crate::{
    GeneratedCrateError, generated_release::ValidRelease, generated_source::Inputs,
    generated_tree::TreeSnapshot,
};

pub(crate) fn validate(
    snapshot: &TreeSnapshot,
    release: &ValidRelease,
) -> Result<(), GeneratedCrateError> {
    let mut files = BTreeMap::from([
        (PathBuf::from(".musubi-generated"), None),
        (PathBuf::from("Cargo.toml"), None),
        (PathBuf::from("musubi-release.json"), None),
        (PathBuf::from("src/lib.rs"), None),
    ]);
    for contract in &release.contracts {
        let base = PathBuf::from("src").join(&contract.rust_module);
        let mut facade = Inputs::default();
        facade.modules.insert("types".into(), None);
        facade
            .modules
            .insert("__evm_artifact".into(), Some("artifact.rs".into()));
        files.insert(base.join("types.rs"), Some(Inputs::default()));
        if contract.kind == musubi_core::ArtifactKind::Deployable {
            facade
                .modules
                .insert("__evm_constructor".into(), Some("constructor.rs".into()));
            files.insert(base.join("constructor.rs"), Some(Inputs::default()));
        }
        let mut artifact = Inputs::default();
        for asset in [
            Some("abi.json"),
            contract
                .creation_template_keccak256
                .as_ref()
                .map(|_| "creation.bin"),
            contract
                .runtime_template_keccak256
                .as_ref()
                .map(|_| "runtime.bin"),
        ]
        .into_iter()
        .flatten()
        {
            files.insert(PathBuf::from("assets").join(&contract.id).join(asset), None);
            artifact
                .assets
                .insert(format!("../../assets/{}/{asset}", contract.id));
        }
        files.insert(base.join("artifact.rs"), Some(artifact));
        let abi = snapshot.file(&format!("assets/{}/abi.json", contract.id))?;
        for owner in musubi_codegen::semantic_abi_modules(abi).map_err(|error| {
            if error.diagnostic.code == musubi_codegen::DiagnosticCode::LimitExceeded {
                GeneratedCrateError::LimitExceeded
            } else {
                GeneratedCrateError::InvalidLayout
            }
        })? {
            files.insert(base.join(&owner.path), Some(Inputs::default()));
            facade.modules.insert(owner.module, Some(owner.path));
        }
        for (relative, module) in leaves(snapshot, &base, "shared_types")? {
            files.insert(base.join(&relative), Some(Inputs::default()));
            facade.modules.insert(module, Some(relative));
        }
        crate::generated_types::contract(snapshot, &base, &facade.modules)?;
        files.insert(base.join("mod.rs"), Some(facade));
    }
    for typed in &release.typed_data {
        typed_files(snapshot, typed, &mut files)?;
    }
    if files.keys().ne(snapshot.files.keys()) {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    for (path, inputs) in files {
        if let Some(inputs) = inputs {
            crate::generated_source::validate(&snapshot.files[&path], &inputs)?;
        }
    }
    Ok(())
}

fn typed_files(
    snapshot: &TreeSnapshot,
    authority: &crate::generated_release::ValidTypedData,
    files: &mut BTreeMap<PathBuf, Option<Inputs>>,
) -> Result<(), GeneratedCrateError> {
    let base = PathBuf::from("src").join(&authority.rust_module);
    let mut facade = Inputs::default();
    facade
        .modules
        .insert("__evm_descriptors".into(), Some("descriptors.rs".into()));
    files.insert(base.join("descriptors.rs"), Some(Inputs::default()));
    let mut types = BTreeSet::new();
    let mut exports = BTreeSet::new();
    for (relative, module) in leaves(snapshot, &base, "types")? {
        let bytes = snapshot
            .files
            .get(&base.join(&relative))
            .ok_or(GeneratedCrateError::InvalidLayout)?;
        let source = std::str::from_utf8(bytes).map_err(invalid)?;
        let name = source
            .strip_prefix("//! Owned EIP-712 `")
            .and_then(|source| source.split_once("` value, conversion, and hashing methods.\n"))
            .map(|(name, _)| name)
            .filter(|name| musubi_core::is_canonical_solidity_identifier(name))
            .ok_or(GeneratedCrateError::InvalidLayout)?;
        let identity = format!("{}:{}:{name}", authority.id.len(), authority.id);
        let expected =
            musubi_codegen::semantic_owner("types", name, identity.as_bytes()).map_err(invalid)?;
        if expected.path != relative || expected.module != module || !types.insert(name.to_owned())
        {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        let definitions = crate::generated_types::definitions(bytes)?;
        if definitions.len() != 1 {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        exports.insert((
            module.clone(),
            definitions
                .into_iter()
                .next()
                .ok_or(GeneratedCrateError::InvalidLayout)?,
        ));
        files.insert(base.join(&relative), Some(Inputs::default()));
        facade.modules.insert(module, Some(relative));
    }
    if types.is_empty() || !authority.required_types.is_subset(&types) {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    crate::generated_types::typed_exports(
        snapshot.file(&format!("src/{}/mod.rs", authority.rust_module))?,
        &exports,
    )?;
    files.insert(base.join("mod.rs"), Some(facade));
    Ok(())
}

/// Owner leaf names are bounded ASCII stems with the sole 128-bit lowercase identity suffix.
fn leaves(
    snapshot: &TreeSnapshot,
    base: &Path,
    kind: &str,
) -> Result<Vec<(String, String)>, GeneratedCrateError> {
    let directory = base.join(kind);
    let mut output = Vec::new();
    for (path, _) in snapshot
        .files
        .range(directory.clone()..)
        .take_while(|(path, _)| path.starts_with(&directory))
    {
        let basename = path
            .strip_prefix(&directory)
            .map_err(invalid)?
            .to_str()
            .ok_or(GeneratedCrateError::InvalidLayout)?;
        let basename = basename
            .strip_suffix(".rs")
            .ok_or(GeneratedCrateError::InvalidLayout)?;
        let (stem, digest) = basename
            .rsplit_once('_')
            .ok_or(GeneratedCrateError::InvalidLayout)?;
        if stem.is_empty()
            || stem.len() > 48
            || !stem
                .bytes()
                .all(|byte| byte == b'_' || byte.is_ascii_lowercase() || byte.is_ascii_digit())
            || digest.len() != 32
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        output.push((
            format!("{kind}/{basename}.rs"),
            format!("__evm_{kind}_{basename}"),
        ));
    }
    Ok(output)
}

fn invalid<T>(_: T) -> GeneratedCrateError {
    GeneratedCrateError::InvalidLayout
}
