//! Focused tests for the sole supported config-v4 wire schema.

use std::path::Path;

use crate::manifest::{CONFIG_V4, ParseError, parse};

const HEADER: &str = r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[package]
name = "manifest-test-sdk"
version = "0.1.0"
edition = "2024"

[generation]
output = "generated/manifest-test-sdk"
"#;

fn contract_manifest(extra: &str) -> Vec<u8> {
    format!(
        r#"{HEADER}
[[contracts]]
id = "vault"
kind = "deployable"
abi = "abi.json"
creation_bytecode = "creation.json"
{extra}"#,
    )
    .into_bytes()
}

#[test]
fn config_v4_is_the_only_supported_manifest_schema() {
    let manifest = parse(&contract_manifest("")).unwrap();
    assert_eq!(manifest.schema_version, CONFIG_V4);
    assert_eq!(
        manifest.generation.output.as_path(),
        Path::new("generated/manifest-test-sdk")
    );

    for old in ["musubi/config-v1", "musubi/config-v2", "musubi/config-v3"] {
        let raw = String::from_utf8(contract_manifest("")).unwrap();
        assert_eq!(
            parse(raw.replace(CONFIG_V4, old).as_bytes()),
            Err(ParseError::UnsupportedSchema)
        );
        assert_eq!(
            parse(format!("schema_version = {old:?}\n").as_bytes()),
            Err(ParseError::UnsupportedSchema)
        );
    }
}

#[test]
fn config_v4_requires_every_top_level_authority() {
    for raw in [
        HEADER.replace("schema_version = \"musubi/config-v4\"\n", ""),
        HEADER.replace("[runtime]\nsource = \"path\"\nroot = \"../..\"\n", ""),
        HEADER.replace(
            "[package]\nname = \"manifest-test-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n",
            "",
        ),
        HEADER.replace(
            "[generation]\noutput = \"generated/manifest-test-sdk\"\n",
            "",
        ),
        HEADER.replace("output = \"generated/manifest-test-sdk\"\n", ""),
    ] {
        assert!(parse(raw.as_bytes()).is_err(), "accepted {raw:?}");
    }
}

#[test]
fn config_v4_rejects_unknown_and_runtime_source_fields() {
    for extra in [
        "unknown = true\n",
        "runtime_git_url = \"ssh://git@example.invalid/repo\"\n",
        "runtime_git_revision = \"1111111111111111111111111111111111111111\"\n",
        "development_runtime_path = \"../runtime\"\n",
    ] {
        assert!(
            parse(&contract_manifest(extra)).is_err(),
            "accepted {extra:?}"
        );
    }

    for field in [
        "unknown = true",
        "runtime = { distribution = \"bundled\" }",
        "runtime_git_url = \"ssh://git@example.invalid/repo\"",
        "runtime_git_revision = \"1111111111111111111111111111111111111111\"",
        "development_runtime_path = \"../runtime\"",
        "force = true",
        "override = true",
    ] {
        for table in ["", "[package]\n", "[generation]\n", "[limits]\n"] {
            let raw = if table.is_empty() {
                format!("{field}\n{HEADER}")
            } else if table == "[limits]\n" {
                format!("{HEADER}\n{table}{field}\n")
            } else {
                HEADER.replace(table, &format!("{table}{field}\n"))
            };
            assert!(parse(raw.as_bytes()).is_err(), "accepted {table}{field}");
        }
    }
}

#[test]
fn generation_output_accepts_only_normal_relative_components() {
    for output in [
        "",
        "/absolute",
        ".",
        "..",
        "./sdk",
        "sdk/.",
        "sdk/../other",
        "sdk//other",
        "sdk/",
        "sdk\\other",
        "sdk\u{0}other",
        "sdk\u{1f}other",
        "sdk\u{7f}other",
        "sdk\nother",
        "C:/sdk",
    ] {
        let mut raw: toml::Value = toml::from_str(HEADER).unwrap();
        raw["generation"]["output"] = output.into();
        let raw = toml::to_string(&raw).unwrap();
        assert!(parse(raw.as_bytes()).is_err(), "accepted {output:?}");
    }
}

#[test]
fn required_fields_cannot_be_omitted_and_field_types_are_closed() {
    let original = String::from_utf8(contract_manifest("")).unwrap();
    for line in [
        "name = \"manifest-test-sdk\"",
        "version = \"0.1.0\"",
        "edition = \"2024\"",
        "id = \"vault\"",
        "kind = \"deployable\"",
        "abi = \"abi.json\"",
    ] {
        assert!(
            parse(original.replace(line, "").as_bytes()).is_err(),
            "omitted {line}"
        );
    }
    for value in ["42", "true", "[]", "{}"] {
        let raw = HEADER.replace("\"generated/manifest-test-sdk\"", value);
        assert!(parse(raw.as_bytes()).is_err());
    }
}

#[test]
fn empty_active_manifest_is_rejected_before_input_loading() {
    let root = tempfile::tempdir().unwrap();
    let manifest = root.path().join("musubi.toml");
    std::fs::write(&manifest, HEADER).unwrap();
    assert!(crate::load_inputs(&manifest).is_err());
    assert!(!root.path().join("generated").exists());
}

#[test]
fn contract_and_typed_data_arrays_default_but_unknown_fields_fail() {
    let manifest = parse(HEADER.as_bytes()).unwrap();
    assert!(manifest.contracts.is_empty());
    assert!(manifest.typed_data.is_empty());

    let typed = format!(
        r#"{HEADER}
[[typed_data]]
id = "message"
rust_module = "message"
authority = "closed-schema-v1"
schema = "schema.json"
vectors = "vectors.json"
override = true
"#,
    );
    assert!(parse(typed.as_bytes()).is_err());
}
