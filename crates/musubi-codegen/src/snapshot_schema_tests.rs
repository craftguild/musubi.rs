//! Only the current snapshot schema carries exact, mandatory provenance and lengths.

use crate::{SnapshotError, parse_snapshot, snapshot_json};

#[test]
fn only_current_snapshot_schema_is_accepted_without_normalization() {
    for version in [1, 2, 3, 5] {
        let raw = format!("{{\"schema_version\":{version}}}");
        assert_eq!(
            parse_snapshot(raw.as_bytes()),
            Err(SnapshotError::UnsupportedSchema)
        );
    }
    let snapshot = crate::snapshot_validation_tests::contract_snapshot();
    assert_eq!(snapshot.schema_version, 4);
    assert_eq!(
        parse_snapshot(&snapshot_json(&snapshot).unwrap()).unwrap(),
        snapshot
    );
}

#[test]
fn snapshot_manifest_evidence_is_required_canonical_and_nonzero() {
    let snapshot = crate::snapshot_validation_tests::contract_snapshot();
    let original = serde_json::to_value(snapshot).unwrap();
    for field in ["manifest_schema", "manifest_sha256"] {
        let mut document = original.clone();
        document.as_object_mut().unwrap().remove(field);
        assert!(parse_snapshot(&serde_json::to_vec(&document).unwrap()).is_err());
    }
    for value in ["musubi/config-v2", "", "musubi/config-v4 "] {
        let mut document = original.clone();
        document["manifest_schema"] = value.into();
        assert!(parse_snapshot(&serde_json::to_vec(&document).unwrap()).is_err());
    }
    for value in [
        "0x".to_owned() + &"00".repeat(32),
        "0x".to_owned() + &"AA".repeat(32),
        "11".repeat(32),
    ] {
        let mut document = original.clone();
        document["manifest_sha256"] = value.into();
        assert!(parse_snapshot(&serde_json::to_vec(&document).unwrap()).is_err());
    }
}
