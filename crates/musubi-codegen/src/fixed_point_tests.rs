//! Solidity fixed-point zero-decimal parsing and generated-surface tests.

use std::fs;

use musubi_core::AbiType;
use serde_json::json;

use crate::{HardLimits, abi_parse::parse_abi, build_ir, load_inputs, render};

#[test]
fn parser_accepts_explicit_zero_decimal_fixed_types() {
    let abi = parse_abi(
        &[json!({
            "type":"function", "name":"fixedZero", "stateMutability":"pure",
            "inputs":[
                {"name":"unsigned","type":"ufixed8x0"},
                {"name":"signed","type":"fixed256x0"}
            ],
            "outputs":[]
        })],
        HardLimits::V1,
    )
    .unwrap();
    assert_eq!(abi.functions[0].inputs[0].ty, AbiType::UFixed(8, 0));
    assert_eq!(abi.functions[0].inputs[1].ty, AbiType::Fixed(256, 0));
}

#[test]
fn generated_surface_retains_zero_decimal_fixed_types() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("abi.json"),
        br#"[{"type":"function","name":"fixedZero","stateMutability":"pure","inputs":[{"name":"unsigned","type":"ufixed8x0"},{"name":"signed","type":"fixed256x0"}],"outputs":[]}]"#,
    )
    .unwrap();
    fs::write(root.path().join("creation.json"), br#""0x6000""#).unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."
[generation]
output = "sdk"

[package]
name = "fixed-zero-sdk"
version = "0.1.0"
edition = "2024"
[[contracts]]
kind = "deployable"
id = "fixed_zero"
abi = "abi.json"
creation_bytecode = "creation.json"
"#,
    )
    .unwrap();
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let rust = tree
        .files()
        .filter(|file| file.path.extension().is_some_and(|value| value == "rs"))
        .map(|file| String::from_utf8(file.contents).unwrap())
        .collect::<String>();
    assert!(rust.contains("UFixed<8, 0>"));
    assert!(rust.contains("Fixed<256, 0>"));
    assert!(rust.contains("AbiType::UFixed(8, 0)"));
    assert!(rust.contains("AbiType::Fixed(256, 0)"));
}
