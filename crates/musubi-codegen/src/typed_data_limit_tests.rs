//! Typed-data-specific hard-limit and override regression tests.

use std::{fs, path::Path};

use crate::{build_ir, load_inputs};

fn write_fixture(root: &Path, limit: &str) {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../external-acceptance/typed-data-array-v1");
    let manifest = fs::read_to_string(source.join("musubi.toml")).unwrap();
    fs::write(
        root.join("musubi.toml"),
        format!("{manifest}\n[limits]\n{limit}\n"),
    )
    .unwrap();
    fs::copy(source.join("schema.json"), root.join("schema.json")).unwrap();
    fs::copy(source.join("vectors.json"), root.join("vectors.json")).unwrap();
}

#[test]
fn schema_and_vector_byte_overrides_are_distinct_and_enforced() {
    for limit in ["typed_data_schema_bytes = 1", "typed_data_vector_bytes = 1"] {
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path(), limit);
        assert!(load_inputs(&root.path().join("musubi.toml")).is_err());
    }
}

#[test]
fn typed_data_overrides_cannot_raise_versioned_maxima() {
    for limit in [
        "typed_data_schema_bytes = 8388609",
        "typed_data_vector_bytes = 16777217",
        "typed_data_entries = 257",
        "typed_data_types = 4097",
        "typed_data_vectors = 4097",
        "typed_data_fields = 65537",
        "typed_data_fixed_array_elements = 65537",
    ] {
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path(), limit);
        assert!(load_inputs(&root.path().join("musubi.toml")).is_err());
    }
}

#[test]
fn typed_data_entry_override_is_applied_independently() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), "typed_data_entries = 1");
    let manifest_path = root.path().join("musubi.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str(
        r#"
[[typed_data]]
id = "group_two"
rust_module = "group_two"
authority = "closed-schema-v1"
schema = "schema.json"
vectors = "vectors.json"
"#,
    );
    fs::write(&manifest_path, manifest).unwrap();
    assert!(load_inputs(&manifest_path).is_err());
}

#[test]
fn vector_count_override_is_applied_before_vector_recomputation() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), "typed_data_vectors = 1");
    let path = root.path().join("vectors.json");
    let mut vectors: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let duplicate = vectors["vectors"][0].clone();
    vectors["vectors"].as_array_mut().unwrap().push(duplicate);
    fs::write(path, serde_json::to_vec(&vectors).unwrap()).unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(build_ir(inputs).is_err());
}

#[test]
fn type_root_and_field_overrides_are_enforced_before_evidence_work() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), "typed_data_fields = 2");
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(build_ir(inputs).is_err());

    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), "typed_data_types = 1");
    fs::write(
        root.path().join("schema.json"),
        br#"{
          "schema_version":"musubi/typed-data-schema-v1",
          "types":[{"name":"Payload","fields":[{"name":"value","type":"string"}]}],
          "roots":[
            {"kind":"struct_hash","type":"Payload"},
            {"kind":"typed_data","type":"Payload","domain":"Payload"}
          ]
        }"#,
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(build_ir(inputs).is_err());
}

#[test]
fn named_struct_chains_obey_a_lowered_nesting_limit() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), "nesting = 1");
    fs::write(
        root.path().join("schema.json"),
        br#"{
          "schema_version":"musubi/typed-data-schema-v1",
          "types":[
            {"name":"Node0","fields":[{"name":"value","type":"uint256"}]},
            {"name":"Node1","fields":[{"name":"child","type":"Node0"}]},
            {"name":"Node2","fields":[{"name":"child","type":"Node1"}]}
          ],
          "roots":[{"kind":"struct_hash","type":"Node2"}]
        }"#,
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let error = build_ir(inputs).unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "typed-data graph exceeds the effective nesting limit"
    }));
}

#[test]
fn typed_fixed_arrays_obey_the_cumulative_element_limit() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), "typed_data_fixed_array_elements = 3");
    fs::write(
        root.path().join("schema.json"),
        br#"{
          "schema_version":"musubi/typed-data-schema-v1",
          "types":[{"name":"Payload","fields":[{"name":"values","type":"uint256[2][2]"}]}],
          "roots":[{"kind":"struct_hash","type":"Payload"}]
        }"#,
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(build_ir(inputs).is_err());
}

#[test]
fn typed_numeric_type_parts_require_canonical_ascii_decimal() {
    for invalid in ["uint+8", "uint08", "bytes01", "uint256[+2]", "uint256[02]"] {
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path(), "typed_data_vectors = 1");
        fs::write(
            root.path().join("schema.json"),
            format!(
                r#"{{
                  "schema_version":"musubi/typed-data-schema-v1",
                  "types":[{{"name":"Payload","fields":[{{"name":"value","type":"{invalid}"}}]}}],
                  "roots":[{{"kind":"struct_hash","type":"Payload"}}]
                }}"#,
            ),
        )
        .unwrap();
        let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
        assert!(build_ir(inputs).is_err(), "accepted {invalid}");
    }
}

#[test]
fn typed_domain_roots_must_generate_distinct_digest_methods() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), "typed_data_vectors = 1");
    fs::write(
        root.path().join("schema.json"),
        br#"{
          "schema_version":"musubi/typed-data-schema-v1",
          "types":[
            {"name":"AB","fields":[{"name":"chainId","type":"uint256"}]},
            {"name":"A_B","fields":[{"name":"chainId","type":"uint256"}]},
            {"name":"Message","fields":[{"name":"value","type":"uint256"}]}
          ],
          "roots":[
            {"kind":"typed_data","type":"Message","domain":"AB"},
            {"kind":"typed_data","type":"Message","domain":"A_B"}
          ]
        }"#,
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(build_ir(inputs).is_err());
}

#[test]
fn struct_hash_vectors_require_domain_fields_to_be_omitted() {
    for field in ["domain_type", "domain", "domain_type_string"] {
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path(), "typed_data_vectors = 1");
        let path = root.path().join("vectors.json");
        let mut vectors: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        vectors["vectors"][0]
            .as_object_mut()
            .unwrap()
            .insert(field.into(), serde_json::Value::Null);
        fs::write(path, serde_json::to_vec(&vectors).unwrap()).unwrap();
        let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
        assert!(build_ir(inputs).is_err(), "accepted present-null {field}");
    }
}
