//! Recorded generator replay and exact drift rejection without Cargo or network execution.

use std::{fs, path::Path};

use crate::{check_generated, render, test_generator::info};

fn fixture() -> (tempfile::TempDir, crate::GenerationIr, crate::OutputTarget) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"
[runtime]
source = "path"
root = "."
[package]
name = "recorded-test"
version = "0.1.0"
edition = "2024"
[generation]
output = "sdk"
[[contracts]]
id = "api"
kind = "interface"
abi = "abi.json"
"#,
    )
    .unwrap();
    fs::write(root.path().join("abi.json"), b"[]").unwrap();
    let inputs = crate::load_inputs(&root.path().join("musubi.toml")).unwrap();
    let output = inputs.output.clone();
    let ir = crate::build_ir(inputs).unwrap();
    (root, ir, output)
}

fn materialize(tree: &crate::GeneratedTree, root: &Path) {
    for file in tree.files() {
        let path = root.join(file.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, file.contents).unwrap();
    }
}

#[test]
fn recorded_generator_replay_preserves_original_bytes_and_reports_both_values() {
    let (root, ir, output) = fixture();
    let original = info();
    let tree = render(&ir, &original).unwrap();
    assert_eq!(tree, render(&ir, &original).unwrap());
    materialize(&tree, &root.path().join("sdk"));
    let mut current = original.clone();
    current.revision = "1234567890abcdef1234567890abcdef12345678-dirty".into();
    current.git_commit = Some("1234567890abcdef1234567890abcdef12345678".into());
    current.dirty = Some(true);
    current.build.host = "another-host".into();
    let report = check_generated(&ir, &current, &output).unwrap();
    assert!(report.is_clean());
    assert_eq!(report.notices.len(), 1);
    assert_eq!(report.notices[0].severity, crate::Severity::Info);
    for generator in [&original, &current] {
        assert!(
            report.notices[0]
                .message
                .contains(&serde_json::to_string(generator).unwrap())
        );
    }
    assert!(crate::check(&tree, &output).unwrap().is_clean());
    assert_ne!(tree, render(&ir, &current).unwrap());
    let old = crate::build_snapshot(&ir, &original);
    let new = crate::build_snapshot(&ir, &current);
    assert!(
        crate::diff_snapshots(&old, &new)
            .changes
            .iter()
            .all(|change| change.category == crate::CompatibilityCategory::ProvenanceOnly)
    );
}

#[test]
fn recorded_generator_replay_rejects_metadata_inconsistency_and_all_file_drift() {
    for path in [
        "src/lib.rs",
        "src/api/mod.rs",
        "assets/api/abi.json",
        "Cargo.toml",
        "added.txt",
    ] {
        let (root, ir, output) = fixture();
        let tree = render(&ir, &info()).unwrap();
        let sdk = root.path().join("sdk");
        materialize(&tree, &sdk);
        fs::write(sdk.join(path), b"changed").unwrap();
        assert!(
            !check_generated(&ir, &info(), &output).unwrap().is_clean(),
            "{path}"
        );
        assert_eq!(fs::read(sdk.join(path)).unwrap(), b"changed");
    }
    for mutation in [
        "missing",
        "invalid",
        "unknown-field",
        "json-only",
        "old-schema",
        "duplicate",
    ] {
        let (root, ir, output) = fixture();
        let tree = render(&ir, &info()).unwrap();
        let sdk = root.path().join("sdk");
        materialize(&tree, &sdk);
        let path = sdk.join("musubi-release.json");
        let mut release: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match mutation {
            "missing" => {
                release.as_object_mut().unwrap().remove("generator");
            }
            "invalid" => release["generator"]["revision"] = "invalid".into(),
            "unknown-field" => release["generator"]["extra"] = true.into(),
            "json-only" => release["generator"]["version"] = "0.2.0".into(),
            "old-schema" => release["schema_version"] = "musubi/release-v3".into(),
            _ => {}
        }
        let mut bytes = serde_json::to_vec(&release).unwrap();
        if mutation == "duplicate" {
            bytes.splice(1..1, b"\"generator\":{},".iter().copied());
        }
        fs::write(&path, &bytes).unwrap();
        let result = check_generated(&ir, &info(), &output);
        assert!(result.is_err() || !result.unwrap().is_clean(), "{mutation}");
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn recorded_generator_replay_rejects_current_runtime_and_missing_source_drift() {
    let (root, mut ir, output) = fixture();
    let tree = render(&ir, &info()).unwrap();
    let sdk = root.path().join("sdk");
    materialize(&tree, &sdk);
    ir.runtime = crate::RuntimeSource::Path {
        root: "../different".into(),
    };
    assert!(!check_generated(&ir, &info(), &output).unwrap().is_clean());
    fs::remove_file(sdk.join("src/api/mod.rs")).unwrap();
    let report = check_generated(&ir, &info(), &output).unwrap();
    assert!(report.added.contains(&"src/api/mod.rs".into()));
}

#[test]
fn checking_identity_length_cannot_exhaust_the_recorded_output_budget() {
    let (root, mut ir, output) = fixture();
    let original = info();
    let tree = render(&ir, &original).unwrap();
    ir.limits.generated_bytes = tree.files().map(|file| file.contents.len()).sum();
    assert!(render(&ir, &original).is_ok());
    materialize(&tree, &root.path().join("sdk"));
    let mut current = original;
    current.build.host = "long-host-".repeat(20).into();
    current.validate().unwrap();
    assert!(render(&ir, &current).is_err());
    assert!(check_generated(&ir, &current, &output).unwrap().is_clean());
}
