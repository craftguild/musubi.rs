//! Generated constructor payability boundary tests.

use std::{fs, path::Path};

use crate::{build_ir, load_inputs, render, validate_generated_cargo};

const VALUE_GUARD: &str = "if value != musubi_core::U256::ZERO { return Err(musubi_runtime::CodecError::UnexpectedValue.into()); }";

#[test]
fn generated_constructor_value_policy_is_closed_and_standalone_compiles() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    write_fixture(root.path());
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let mut tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();

    let absent = source(&tree, "src/absent/constructor.rs");
    let explicit = source(&tree, "src/explicit_nonpayable/constructor.rs");
    let payable = source(&tree, "src/payable/constructor.rs");
    assert!(absent.contains(VALUE_GUARD));
    assert!(explicit.contains(VALUE_GUARD));
    assert!(!payable.contains(VALUE_GUARD));

    tree.insert(
        "tests/constructor_value.rs",
        GENERATED_TEST.as_bytes().to_vec(),
    )
    .unwrap();
    tree.seal().unwrap();
    validate_generated_cargo(&tree, &root.path().join("sdk"), root.path()).unwrap();
}

fn source(tree: &crate::GeneratedTree, path: &str) -> String {
    String::from_utf8(
        tree.files()
            .find(|file| file.path == Path::new(path))
            .unwrap()
            .contents,
    )
    .unwrap()
}

fn write_fixture(root: &Path) {
    fs::write(root.join("absent.json"), "[]").unwrap();
    fs::write(
        root.join("explicit.json"),
        r#"[{"type":"constructor","stateMutability":"nonpayable","inputs":[]}]"#,
    )
    .unwrap();
    fs::write(
        root.join("payable.json"),
        r#"[{"type":"constructor","stateMutability":"payable","inputs":[]}]"#,
    )
    .unwrap();
    fs::write(root.join("creation.json"), r#""0x6000""#).unwrap();
    fs::write(
        root.join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[generation]
output = "sdk"

[package]
name = "generated-constructor-sdk"
version = "0.1.0"
edition = "2024"

[[contracts]]
id = "absent"
kind = "deployable"
abi = "absent.json"
creation_bytecode = "creation.json"

[[contracts]]
id = "explicit_nonpayable"
kind = "deployable"
abi = "explicit.json"
creation_bytecode = "creation.json"

[[contracts]]
id = "payable"
kind = "deployable"
abi = "payable.json"
creation_bytecode = "creation.json"
"#,
    )
    .unwrap();
}

const GENERATED_TEST: &str = r"
use musubi_core::U256;
use musubi_runtime::{CodecError, DeploymentError, LibraryAddresses};
use generated_constructor_sdk::{
    absent::Absent, explicit_nonpayable::ExplicitNonpayable, payable::Payable,
};

#[test]
fn generated_constructor_value_policy() {
    let libraries = LibraryAddresses::new();
    let one = U256::from_u128(1).unwrap();
    let unexpected = Err(DeploymentError::Codec(CodecError::UnexpectedValue));
    assert_eq!(Absent::deployment(&libraries, one), unexpected);
    assert_eq!(
        ExplicitNonpayable::deployment(&libraries, one),
        Err(DeploymentError::Codec(CodecError::UnexpectedValue))
    );
    let request = Payable::deployment(&libraries, one).unwrap();
    assert_eq!(request.value, one);
    assert!(request.to.is_none());
}
";
