//! Raw ABI input and canonical generated-asset byte budgets through load, IR, and render.

use std::{fs, path::Path};

use crate::{DiagnosticCode, GenerationIr, HardLimits, build_ir, load_inputs, render};

fn fixture(root: &Path, events: usize, limits: &str) -> Vec<u8> {
    let fields = (0..16)
        .map(|index| {
            serde_json::json!({
                "name": format!("field_{index}_{}", "x".repeat(64)), "type": "uint256"
            })
        })
        .collect::<Vec<_>>();
    let abi = (0..events)
        .map(|index| {
            serde_json::json!({"type": "event", "name": format!("E{index}"), "inputs": fields})
        })
        .collect::<Vec<_>>();
    let raw = serde_json::to_vec(&abi).unwrap();
    fs::write(root.join("abi.json"), &raw).unwrap();
    fs::write(
        root.join("musubi.toml"),
        format!(
            r#"schema_version = "musubi/config-v4"
[runtime]
source = "path"
root = "../.."
[package]
name = "canonical-budget-sdk"
version = "0.1.0"
edition = "2024"
[generation]
output = "sdk"
[[contracts]]
id = "many_events"
kind = "interface"
abi = "abi.json"
{limits}
"#
        ),
    )
    .unwrap();
    raw
}

fn load(root: &Path) -> GenerationIr {
    build_ir(load_inputs(&root.join("musubi.toml")).unwrap()).unwrap()
}

#[test]
fn large_canonical_abi_renders_above_the_raw_input_cap() {
    let root = tempfile::tempdir().unwrap();
    let raw = fixture(root.path(), HardLimits::V1.abi_items, "");
    assert_eq!(raw.len(), 6_818_731);
    assert!(raw.len() <= HardLimits::V1.abi_bytes);
    let ir = load(root.path());
    let canonical = &ir.contracts[0].canonical_abi;
    assert_eq!(canonical.len(), 9_165_741);
    assert!(canonical.len() > ir.limits.abi_bytes);
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let (_, asset) = tree
        .file_entries()
        .find(|(path, _)| *path == Path::new("assets/many_events/abi.json"))
        .unwrap();
    assert_eq!(asset, canonical);
}

#[test]
fn lowered_raw_abi_cap_allows_canonical_expansion() {
    let root = tempfile::tempdir().unwrap();
    let raw = fixture(root.path(), 1, "");
    fixture(
        root.path(),
        1,
        &format!("[limits]\nabi_bytes = {}", raw.len()),
    );
    let ir = load(root.path());
    assert!(ir.contracts[0].canonical_abi.len() > ir.limits.abi_bytes);
    assert!(render(&ir, &crate::test_generator::info()).is_ok());

    fixture(
        root.path(),
        1,
        &format!("[limits]\nabi_bytes = {}", raw.len() - 1),
    );
    let error = load_inputs(&root.path().join("musubi.toml")).unwrap_err();
    assert_eq!(
        error.diagnostics.iter().next().unwrap().code,
        DiagnosticCode::LimitExceeded
    );
    assert!(!root.path().join("sdk").exists());
}

#[test]
fn canonical_asset_and_aggregate_output_keep_the_generated_byte_cap() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), 1, "");
    let mut ir = load(root.path());
    let canonical_bytes = ir.contracts[0].canonical_abi.len();
    ir.limits.generated_bytes = canonical_bytes - 1;
    assert_eq!(
        render(&ir, &crate::test_generator::info())
            .unwrap_err()
            .diagnostic
            .code,
        DiagnosticCode::LimitExceeded
    );
    ir.limits.generated_bytes = canonical_bytes;
    assert!(render(&ir, &crate::test_generator::info()).is_err());
}
