//! Event-specific ABI parsing regressions.

use serde_json::json;

use crate::{DiagnosticCode, HardLimits, abi_parse::parse_abi};

#[test]
fn abi_rejects_duplicate_anonymous_event_signatures() {
    let first = json!({
        "type":"event", "name":"Choice", "anonymous":true,
        "inputs":[{"name":"id", "type":"uint256", "indexed":true}]
    });
    for duplicate in [
        first.clone(),
        json!({
            "type":"event", "name":"Choice", "anonymous":true,
            "inputs":[{"name":"id", "type":"uint256", "indexed":false}]
        }),
    ] {
        let error = parse_abi(&[first.clone(), duplicate], HardLimits::V1).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
    }
}

#[test]
fn abi_rejects_reserved_standard_error_selectors() {
    for (name, ty) in [("Error", "string"), ("Panic", "uint256")] {
        let error = parse_abi(
            &[json!({
                "type":"error", "name":name,
                "inputs":[{"name":"value", "type":ty}]
            })],
            HardLimits::V1,
        )
        .unwrap_err();
        assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
    }
}
