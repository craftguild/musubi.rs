//! Closed ABI type grammar and resource-limit regression tests.

use serde_json::json;

use crate::{HardLimits, abi_parse::parse_abi};

fn single_input(ty: &str) -> Vec<serde_json::Value> {
    vec![json!({
        "type":"function", "name":"accept", "stateMutability":"pure",
        "inputs":[{"name":"value", "type":ty}], "outputs":[]
    })]
}

#[test]
fn abi_fixed_arrays_enforce_the_cumulative_v1_limit() {
    assert!(parse_abi(&single_input("uint256[256][256]"), HardLimits::V1).is_ok());
    assert!(parse_abi(&single_input("uint256[257][256]"), HardLimits::V1).is_err());
    assert!(
        parse_abi(
            &single_input("uint256[18446744073709551615][2]"),
            HardLimits::V1,
        )
        .is_err()
    );

    let mut lowered = HardLimits::V1;
    lowered.abi_fixed_array_elements = 3;
    assert!(parse_abi(&single_input("uint256[3]"), lowered).is_ok());
    assert!(parse_abi(&single_input("uint256[2][2]"), lowered).is_err());
}

#[test]
fn abi_numeric_type_parts_require_canonical_ascii_decimal() {
    for invalid in [
        "uint+8",
        "uint08",
        "bytes01",
        "fixed128x018",
        "uint256[+2]",
        "uint256[02]",
    ] {
        assert!(
            parse_abi(&single_input(invalid), HardLimits::V1).is_err(),
            "accepted {invalid}"
        );
    }
}

#[test]
fn abi_component_limit_is_checked_during_recursive_parsing() {
    let input = vec![json!({
        "type":"function", "name":"accept", "stateMutability":"pure", "outputs":[],
        "inputs":[{"name":"value", "type":"tuple", "components":[
            {"name":"a", "type":"uint256"},
            {"name":"b", "type":"uint256"}
        ]}]
    })];
    let mut limits = HardLimits::V1;
    limits.abi_components = 2;
    assert!(parse_abi(&input, limits).is_err());
}
