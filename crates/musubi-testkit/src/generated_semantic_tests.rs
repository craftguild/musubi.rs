//! Hash-aligned hostile inputs still fail source and semantic module inspection on every leaf kind.

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    generated_inventory_tests::fixture, generated_test_fixture::reseal, inspect_generated_crate,
};

fn semantic_fixture() -> (tempfile::TempDir, Vec<PathBuf>) {
    let (root, mut release) = fixture();
    let abi = serde_json::json!([
        {"type":"function","name":"read","stateMutability":"view","inputs":[],"outputs":[]},
        {"type":"event","name":"Changed","anonymous":false,"inputs":[]},
        {"type":"error","name":"Rejected","inputs":[]}
    ]);
    let bytes = serde_json::to_vec(&abi).unwrap();
    release["contracts"][0]["canonical_abi_sha256"] =
        hex::encode(musubi_core::sha256(&bytes)).into();
    fs::write(root.path().join("assets/vault/abi.json"), &bytes).unwrap();
    fs::write(
        root.path().join("musubi-release.json"),
        serde_json::to_vec(&release).unwrap(),
    )
    .unwrap();
    let base = root.path().join("src/vault_sdk");
    let mut facade = fs::read_to_string(base.join("mod.rs")).unwrap();
    let mut exports = String::from("//! fixture\n");
    let mut owners = musubi_codegen::semantic_abi_modules(&bytes).unwrap();
    owners.push(
        musubi_codegen::semantic_owner("shared_types", "Shared", b"struct Vault.Shared").unwrap(),
    );
    let mut paths = vec![
        PathBuf::from("src/vault_sdk/artifact.rs"),
        PathBuf::from("src/vault_sdk/constructor.rs"),
    ];
    for owner in owners {
        let name = if owner.path.starts_with("operations/") {
            None
        } else if owner.path.starts_with("events/") {
            Some("ChangedEvent")
        } else if owner.path.starts_with("errors/") {
            Some("RejectedError")
        } else {
            Some("Shared")
        };
        let mut source = String::from("//! fixture semantic owner\n");
        if let Some(name) = name {
            writeln!(source, "pub struct {name};").unwrap();
            writeln!(exports, "pub use super::{}::{name};", owner.module).unwrap();
        }
        writeln!(facade, "#[path = {:?}]\nmod {};", owner.path, owner.module).unwrap();
        let path = base.join(&owner.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        paths.push(path.strip_prefix(root.path()).unwrap().into());
    }
    fs::write(base.join("mod.rs"), facade).unwrap();
    fs::write(base.join("types.rs"), exports).unwrap();
    reseal(root.path());
    inspect_generated_crate(root.path()).unwrap();
    (root, paths)
}

#[test]
fn hash_aligned_forbidden_inputs_are_rejected_in_every_contract_owner_kind() {
    let (root, paths) = semantic_fixture();
    for path in paths {
        let path = root.path().join(path);
        let original = fs::read_to_string(&path).unwrap();
        for source in [
            "const LEAK: &str = include_str!(\"/etc/passwd\");\n",
            "macro_rules! leak { () => { include_bytes!(\"/etc/passwd\") } }\n",
            "const LEAK: &str = concat!(r#env!(\"HOME\"));\n",
            "#[cfg_attr(all(), path=\"/etc/passwd\")] mod leak;\n",
            "macro_rules! leak { () => { mod hidden; } }\n",
            "#[path=\"artifact.rs\"] mod duplicate_asset;\n",
        ] {
            fs::write(&path, format!("{original}{source}")).unwrap();
            reseal(root.path());
            musubi_codegen::read_owned_tree(root.path(), musubi_codegen::HardLimits::V1).unwrap();
            assert!(
                inspect_generated_crate(root.path()).is_err(),
                "accepted {}: {source}",
                path.display()
            );
        }
        fs::write(path, original).unwrap();
    }
    reseal(root.path());
    inspect_generated_crate(root.path()).unwrap();
}

#[test]
fn semantic_modules_cannot_escape_inventory_or_change_an_abi_owner_path() {
    let (root, paths) = semantic_fixture();
    let facade_path = root.path().join("src/vault_sdk/mod.rs");
    let original = fs::read_to_string(&facade_path).unwrap();
    for mutation in [
        original.replace("artifact.rs", "../../outside.rs"),
        original.replace("mod __evm_artifact;", "mod __evm_artifact {}"),
        original.replace("pub mod types;", "mod types;"),
    ] {
        fs::write(&facade_path, mutation).unwrap();
        reseal(root.path());
        assert!(inspect_generated_crate(root.path()).is_err());
    }
    fs::write(&facade_path, &original).unwrap();
    let operation = paths
        .iter()
        .find(|path| path.starts_with("src/vault_sdk/operations"))
        .unwrap();
    let old = operation.file_name().unwrap().to_str().unwrap();
    let new = format!("different_{old}");
    fs::rename(
        root.path().join(operation),
        root.path().join(operation.parent().unwrap()).join(&new),
    )
    .unwrap();
    fs::write(facade_path, original.replace(old, &new)).unwrap();
    reseal(root.path());
    musubi_codegen::read_owned_tree(root.path(), musubi_codegen::HardLimits::V1).unwrap();
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn shared_owner_has_one_definition_exported_once_by_the_existing_types_facade() {
    let (root, paths) = semantic_fixture();
    let shared = paths
        .iter()
        .find(|path| path.starts_with("src/vault_sdk/shared_types"))
        .unwrap();
    let path = root.path().join(shared);
    for invalid in [
        "//! no definition\n",
        "pub struct Shared;\npub struct Duplicate;\n",
    ] {
        fs::write(&path, invalid).unwrap();
        reseal(root.path());
        assert!(inspect_generated_crate(root.path()).is_err());
    }
    fs::write(path, "pub struct Shared;\n").unwrap();
    fs::write(
        root.path().join("src/vault_sdk/types.rs"),
        "//! missing exports\n",
    )
    .unwrap();
    reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn typed_data_descriptors_and_each_struct_are_scanned_after_hash_validation() {
    let mut release = crate::generated_release_tests::valid_release();
    let root = crate::generated_release_tests::fixture_root(&mut release);
    let mut paths = vec![root.path().join("src/message/descriptors.rs")];
    paths.extend(
        fs::read_dir(root.path().join("src/message/types"))
            .unwrap()
            .map(|entry| entry.unwrap().path()),
    );
    inspect_generated_crate(root.path()).unwrap();
    for path in paths {
        let original = fs::read_to_string(&path).unwrap();
        fs::write(
            &path,
            format!("{original}const BAD: Option<&str> = option_env!(\"HOME\");\n"),
        )
        .unwrap();
        reseal(root.path());
        musubi_codegen::read_owned_tree(root.path(), musubi_codegen::HardLimits::V1).unwrap();
        assert!(inspect_generated_crate(root.path()).is_err());
        fs::write(path, original).unwrap();
    }
    reseal(root.path());
    inspect_generated_crate(root.path()).unwrap();
    assert!(!Path::new(root.path()).join("Cargo.lock").exists());
}

#[test]
fn hash_aligned_abi_assets_report_structural_resource_limits() {
    let limits = musubi_codegen::HardLimits::V1;
    for bytes in [
        format!("[{}0]", "0,".repeat(limits.abi_items)),
        format!(
            r#"[{{"type":"function","name":"f","inputs":[{{"type":"tuple","components":[{}0]}}]}}]"#,
            "0,".repeat(limits.abi_components)
        ),
    ] {
        let (root, mut release) = fixture();
        release["contracts"][0]["canonical_abi_sha256"] =
            hex::encode(musubi_core::sha256(bytes.as_bytes())).into();
        fs::write(root.path().join("assets/vault/abi.json"), bytes).unwrap();
        fs::write(
            root.path().join("musubi-release.json"),
            serde_json::to_vec(&release).unwrap(),
        )
        .unwrap();
        reseal(root.path());
        musubi_codegen::read_owned_tree(root.path(), limits).unwrap();
        assert_eq!(
            inspect_generated_crate(root.path()).unwrap_err(),
            crate::GeneratedCrateError::LimitExceeded
        );
    }
}
