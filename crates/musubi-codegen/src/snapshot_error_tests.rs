//! Snapshot validation for reserved Solidity error selectors.

use musubi_core::{AbiField, AbiType, ErrorDescriptor};

use crate::snapshot_validation_tests::{contract_snapshot, parse};

#[test]
fn snapshot_rejects_custom_errors_with_reserved_standard_selectors() {
    for (name, ty, selector) in [
        ("Error", AbiType::String, [0x08, 0xc3, 0x79, 0xa0]),
        ("Panic", AbiType::Uint(256), [0x4e, 0x48, 0x7b, 0x71]),
    ] {
        let mut snapshot = contract_snapshot();
        snapshot.contracts[0].abi.errors.push(ErrorDescriptor {
            name: name.into(),
            signature: format!(
                "{name}({})",
                if name == "Error" { "string" } else { "uint256" }
            ),
            selector,
            inputs: vec![AbiField {
                name: "value".into(),
                ty,
                internal_type: None,
            }],
        });
        assert!(parse(&snapshot).is_err());
    }
}
