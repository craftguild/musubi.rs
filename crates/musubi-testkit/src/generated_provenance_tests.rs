//! Release JSON and generated Rust must agree on exact manifest and runtime identities.

use crate::{generated_inventory_tests::fixture, inspect_generated_crate};
use serde_json::{Value, json};
use std::fs;

#[test]
fn manifest_and_runtime_evidence_cannot_drift_from_rust_root() {
    let (root, original) = fixture();
    assert!(inspect_generated_crate(root.path()).is_ok());
    for (pointer, value) in [
        ("/manifest_sha256", json!(format!("0x{}", "88".repeat(32)))),
        ("/manifest_sha256", json!(format!("0x{}", "00".repeat(32)))),
        ("/manifest_sha256", json!(format!("0x{}", "AA".repeat(32)))),
        ("/manifest_sha256", json!("77".repeat(32))),
        ("/manifest_schema", json!("unrecognized")),
        ("/runtime/root", json!("../different-runtime")),
    ] {
        let mut release = original.clone();
        *release.pointer_mut(pointer).unwrap() = value;
        fs::write(
            root.path().join("musubi-release.json"),
            serde_json::to_vec(&release).unwrap(),
        )
        .unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(
            inspect_generated_crate(root.path()).is_err(),
            "accepted {pointer}"
        );
    }
    for field in ["manifest_schema", "manifest_sha256", "typed_data"] {
        let mut release = original.clone();
        release.as_object_mut().unwrap().remove(field);
        fs::write(
            root.path().join("musubi-release.json"),
            serde_json::to_vec(&release).unwrap(),
        )
        .unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(inspect_generated_crate(root.path()).is_err());
    }
    fs::write(
        root.path().join("musubi-release.json"),
        serde_json::to_vec(&original).unwrap(),
    )
    .unwrap();
    let source = fs::read_to_string(root.path().join("src/lib.rs")).unwrap();
    fs::write(
        root.path().join("src/lib.rs"),
        source.replace("new([119,", "new([118,"),
    )
    .unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn release_reader_accepts_no_earlier_schema_or_inferred_module() {
    let (root, original) = fixture();
    for schema in ["musubi/release-v1", "musubi/release-v2"] {
        let mut release = original.clone();
        release["schema_version"] = Value::String(schema.into());
        fs::write(
            root.path().join("musubi-release.json"),
            serde_json::to_vec(&release).unwrap(),
        )
        .unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(inspect_generated_crate(root.path()).is_err());
    }
    for field in ["rust_module", "kind"] {
        let mut release = original.clone();
        release["contracts"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        fs::write(
            root.path().join("musubi-release.json"),
            serde_json::to_vec(&release).unwrap(),
        )
        .unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(inspect_generated_crate(root.path()).is_err());
    }
}
