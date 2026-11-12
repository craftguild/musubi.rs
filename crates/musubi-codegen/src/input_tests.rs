//! Focused adversarial tests for closed manifest and root-confined loading.

use std::fs;

use tempfile::TempDir;

use musubi_core::ArtifactKind;

use crate::{
    ContractConfig, DiagnosticCode, HardLimits, LimitOverrides, Manifest, PackageConfig, Severity,
    TypedDataAuthority, TypedDataConfig, load_inputs,
};

fn write_minimal(root: &TempDir, ast: bool) {
    fs::write(
        root.path().join("abi.json"),
        br#"[{"type":"function","name":"owner","inputs":[],"outputs":[{"name":"","type":"address"}],"stateMutability":"view"}]"#,
    )
    .unwrap();
    fs::write(root.path().join("creation.json"), br#""0x60006000""#).unwrap();
    if ast {
        fs::write(
            root.path().join("ast.json"),
            br#"{"nodeType":"SourceUnit","nodes":[]}"#,
        )
        .unwrap();
    }
}

fn manifest(ast_line: &str, extra: &str) -> String {
    format!(
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[package]
name = "example-sdk"
version = "0.1.0"
edition = "2024"

[generation]
output = "sdk"

[[contracts]]
id = "vault"
kind = "deployable"
abi = "abi.json"
creation_bytecode = "creation.json"
{ast_line}
{extra}
"#
    )
}

#[test]
fn input_loads_and_emits_coherence_once() {
    let root = TempDir::new().unwrap();
    write_minimal(&root, true);
    fs::write(
        root.path().join("musubi.toml"),
        manifest("ast = \"ast.json\"", ""),
    )
    .unwrap();

    let bundle = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let notices: Vec<_> = bundle
        .diagnostics
        .iter()
        .filter(|item| item.code == DiagnosticCode::ExternalArtifactCoherenceUnverified)
        .collect();
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].severity, Severity::Info);
    assert_eq!(
        bundle.contracts[0]
            .creation
            .as_ref()
            .unwrap()
            .template
            .as_ref(),
        [0x60, 0, 0x60, 0]
    );
}

#[test]
fn input_abi_structure_is_bounded_during_loading_with_lowered_limits() {
    for (budget, bytes) in [
        ("abi_items = 1", "[0,0]"),
        ("abi_components = 1", r#"[{"inputs":[0],"outputs":[0]}]"#),
    ] {
        let root = TempDir::new().unwrap();
        write_minimal(&root, false);
        fs::write(
            root.path().join("musubi.toml"),
            manifest("", &format!("[limits]\n{budget}")),
        )
        .unwrap();
        load_inputs(&root.path().join("musubi.toml")).unwrap();
        fs::write(root.path().join("abi.json"), bytes).unwrap();
        let error = load_inputs(&root.path().join("musubi.toml")).unwrap_err();
        assert_eq!(
            error.diagnostics.iter().next().unwrap().code,
            DiagnosticCode::LimitExceeded
        );
        assert!(!root.path().join("sdk").exists());
    }
}

#[test]
fn input_rejects_non_source_unit_ast_roots() {
    for ast in [
        br"[]".as_slice(),
        br#"{"nodeType":"ContractDefinition","nodes":[]}"#.as_slice(),
    ] {
        let root = TempDir::new().unwrap();
        write_minimal(&root, true);
        fs::write(root.path().join("ast.json"), ast).unwrap();
        fs::write(
            root.path().join("musubi.toml"),
            manifest("ast = \"ast.json\"", ""),
        )
        .unwrap();
        let error = load_inputs(&root.path().join("musubi.toml")).unwrap_err();
        assert_eq!(
            error.diagnostics.iter().next().unwrap().code,
            DiagnosticCode::InvalidInput
        );
    }
}

#[test]
fn missing_ast_is_warning_promoted_only_by_policy() {
    let root = TempDir::new().unwrap();
    write_minimal(&root, false);
    fs::write(root.path().join("musubi.toml"), manifest("", "")).unwrap();

    let bundle = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(!bundle.diagnostics.failed(false));
    assert!(bundle.diagnostics.failed(true));
}

#[test]
fn input_rejects_parent_escape_without_disclosing_absolute_path() {
    let root = TempDir::new().unwrap();
    write_minimal(&root, false);
    fs::write(
        root.path().join("musubi.toml"),
        manifest("", "").replace("abi = \"abi.json\"", "abi = \"../secret.json\""),
    )
    .unwrap();

    let error = load_inputs(&root.path().join("musubi.toml")).unwrap_err();
    let diagnostic = error.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code, DiagnosticCode::UnsafePath);
    assert!(!diagnostic.message.contains(root.path().to_str().unwrap()));
}

#[test]
fn input_rejects_unknown_manifest_field() {
    let root = TempDir::new().unwrap();
    write_minimal(&root, false);
    fs::write(
        root.path().join("musubi.toml"),
        manifest("", "typo_field = true"),
    )
    .unwrap();

    let error = load_inputs(&root.path().join("musubi.toml")).unwrap_err();
    assert_eq!(
        error.diagnostics.iter().next().unwrap().code,
        DiagnosticCode::InvalidInput
    );
}

#[test]
fn input_rejects_abi_larger_than_effective_byte_limit() {
    let root = TempDir::new().unwrap();
    write_minimal(&root, false);
    fs::write(
        root.path().join("musubi.toml"),
        manifest("", "[limits]\nabi_bytes = 64"),
    )
    .unwrap();

    let error = load_inputs(&root.path().join("musubi.toml")).unwrap_err();
    assert_eq!(
        error.diagnostics.iter().next().unwrap().code,
        DiagnosticCode::LimitExceeded
    );
}

#[test]
fn input_rejects_ids_outside_syntax_and_effective_length_limits() {
    for contents in [
        manifest(
            "",
            "rust_module = \"vault\"\n\n[limits]\nidentifier_bytes = 3",
        ),
        manifest("", "rust_module = \"vault\"").replace("id = \"vault\"", "id = \"foo-bar\""),
    ] {
        let root = TempDir::new().unwrap();
        write_minimal(&root, false);
        fs::write(root.path().join("musubi.toml"), contents).unwrap();
        assert!(load_inputs(&root.path().join("musubi.toml")).is_err());
    }
}

#[test]
fn typed_data_limit_fields_are_downward_only() {
    for field in [
        "typed_data_entries = 1",
        "typed_data_fixed_array_elements = 1",
    ] {
        let root = TempDir::new().unwrap();
        write_minimal(&root, false);
        fs::write(
            root.path().join("musubi.toml"),
            manifest("", &format!("[limits]\n{field}")),
        )
        .unwrap();
        assert!(load_inputs(&root.path().join("musubi.toml")).is_ok());
    }
}

#[test]
fn abi_fixed_array_override_is_downward_only() {
    let root = TempDir::new().unwrap();
    write_minimal(&root, false);
    let raw = manifest("", "[limits]\nabi_fixed_array_elements = 65536");
    fs::write(root.path().join("musubi.toml"), &raw).unwrap();
    assert_eq!(
        load_inputs(&root.path().join("musubi.toml"))
            .unwrap()
            .limits
            .abi_fixed_array_elements,
        65_536
    );

    fs::write(
        root.path().join("musubi.toml"),
        raw.replace(
            "abi_fixed_array_elements = 65536",
            "abi_fixed_array_elements = 65537",
        ),
    )
    .unwrap();
    assert!(load_inputs(&root.path().join("musubi.toml")).is_err());
}

#[test]
fn contract_and_typed_data_counts_have_independent_caps() {
    let contracts = (0..HardLimits::V1.contracts)
        .map(|index| ContractConfig {
            id: format!("contract_{index}"),
            kind: ArtifactKind::Deployable,
            abi: "abi.json".into(),
            creation_bytecode: Some("creation.json".into()),
            runtime_bytecode: None,
            ast: None,
            metadata: None,
            storage_layout: None,
            rust_module: None,
        })
        .collect();
    let manifest = Manifest {
        runtime: crate::RuntimeSource::Path { root: ".".into() },
        schema_version: "musubi/config-v4".into(),
        generation: crate::manifest::parse(manifest("", "").as_bytes())
            .unwrap()
            .generation,
        package: PackageConfig {
            name: "limits".into(),
            version: "0.1.0".into(),
            edition: "2024".into(),
        },
        contracts,
        typed_data: vec![TypedDataConfig {
            id: "typed_data".into(),
            rust_module: "typed_data".into(),
            authority: TypedDataAuthority::ClosedSchemaV1,
            schema: "schema.json".into(),
            vectors: "vectors.json".into(),
        }],
        limits: LimitOverrides::default(),
    };
    crate::input_manifest::validate_manifest(&manifest, HardLimits::V1).unwrap();
    let mut too_many_typed = manifest.clone();
    let mut second = too_many_typed.typed_data[0].clone();
    second.id = "typed_data_two".into();
    second.rust_module = "typed_data_two".into();
    too_many_typed.typed_data.push(second);
    let mut lowered = HardLimits::V1;
    lowered.typed_data_entries = 1;
    assert!(crate::input_manifest::validate_manifest(&too_many_typed, lowered).is_err());
}

#[test]
fn typed_data_only_manifest_may_omit_contracts() {
    let manifest = crate::manifest::parse(
        br#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[package]
name = "typed-only"
version = "0.1.0"
edition = "2024"

[generation]
output = "sdk"

[[typed_data]]
id = "message"
rust_module = "message"
authority = "closed-schema-v1"
schema = "schema.json"
vectors = "vectors.json"
"#,
    )
    .unwrap();
    assert!(manifest.contracts.is_empty());
    assert_eq!(manifest.typed_data.len(), 1);
    crate::input_manifest::validate_manifest(&manifest, HardLimits::V1).unwrap();
}
