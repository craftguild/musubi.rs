//! Fail-closed snapshot descriptor and fixed-array resource validation.

use musubi_core::{AbiField, AbiType, FunctionDescriptor, StateMutability, keccak256};

use crate::snapshot_validation_tests::{contract_snapshot, parse};

#[test]
fn snapshot_rejects_forged_contract_descriptors_and_inventories() {
    let mut valid = contract_snapshot();
    let signature = "owner(uint256)";
    valid.contracts[0].abi.functions.push(FunctionDescriptor {
        name: "owner".into(),
        signature: signature.into(),
        selector: keccak256(signature.as_bytes()).as_ref()[..4]
            .try_into()
            .unwrap(),
        inputs: vec![AbiField {
            name: "id".into(),
            ty: AbiType::Uint(256),
            internal_type: None,
        }],
        outputs: Vec::new(),
        state_mutability: StateMutability::View,
    });
    assert!(parse(&valid).is_ok());

    let mut bad_width = valid.clone();
    bad_width.contracts[0].abi.functions[0].inputs[0].ty = AbiType::Uint(7);
    assert!(parse(&bad_width).is_err());

    let mut bad_selector = valid.clone();
    bad_selector.contracts[0].abi.functions[0].selector = [0; 4];
    assert!(parse(&bad_selector).is_err());

    let mut at_limit = valid.clone();
    set_array(&mut at_limit, 256, 256, "owner(uint256[256][256])");
    assert!(parse(&at_limit).is_ok());

    let mut overflowing_array = valid.clone();
    set_array(
        &mut overflowing_array,
        usize::MAX,
        2,
        &format!("owner(uint256[{}][2])", usize::MAX),
    );
    assert!(parse(&overflowing_array).is_err());

    let mut oversized_array = valid;
    set_array(&mut oversized_array, 257, 256, "owner(uint256[257][256])");
    assert!(parse(&oversized_array).is_err());

    let mut invalid_fallback = contract_snapshot();
    invalid_fallback.contracts[0].abi.fallback = Some(StateMutability::Pure);
    assert!(parse(&invalid_fallback).is_err());
}

#[test]
fn snapshot_accepts_zero_decimal_fixed_types_and_rejects_over_eighty() {
    let mut valid = contract_snapshot();
    let signature = "owner(ufixed8x0)";
    valid.contracts[0].abi.functions.push(FunctionDescriptor {
        name: "owner".into(),
        signature: signature.into(),
        selector: keccak256(signature.as_bytes()).as_ref()[..4]
            .try_into()
            .unwrap(),
        inputs: vec![AbiField {
            name: "id".into(),
            ty: AbiType::UFixed(8, 0),
            internal_type: None,
        }],
        outputs: Vec::new(),
        state_mutability: StateMutability::View,
    });
    assert!(parse(&valid).is_ok());

    set_input_type(&mut valid, AbiType::Fixed(256, 0), "owner(fixed256x0)");
    assert!(parse(&valid).is_ok());

    set_input_type(&mut valid, AbiType::Fixed(256, 81), "owner(fixed256x81)");
    assert!(parse(&valid).is_err());
}

fn set_input_type(snapshot: &mut crate::CompatibilitySnapshot, ty: AbiType, signature: &str) {
    let function = &mut snapshot.contracts[0].abi.functions[0];
    function.inputs[0].ty = ty;
    function.signature = signature.into();
    function.selector = keccak256(signature.as_bytes()).as_ref()[..4]
        .try_into()
        .unwrap();
}

fn set_array(
    snapshot: &mut crate::CompatibilitySnapshot,
    inner: usize,
    outer: usize,
    signature: &str,
) {
    let function = &mut snapshot.contracts[0].abi.functions[0];
    function.inputs[0].ty = AbiType::FixedArray(
        Box::new(AbiType::FixedArray(Box::new(AbiType::Uint(256)), inner)),
        outer,
    );
    function.signature = signature.into();
    function.selector = keccak256(signature.as_bytes()).as_ref()[..4]
        .try_into()
        .unwrap();
}
