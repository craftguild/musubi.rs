//! Typed-data compatibility category tests.

use musubi_core::{FixedBytes, TypedDataField, TypedDataRoot, TypedDataStruct, TypedDataType};

use crate::{
    CompatibilityCategory, CompatibilitySnapshot, TypedDataExternalEvidence, TypedDataRootEvidence,
    TypedDataSnapshot, diff_snapshots,
};

fn snapshot() -> CompatibilitySnapshot {
    let types = vec![TypedDataStruct {
        name: "Message".into(),
        fields: vec![TypedDataField {
            name: "body".into(),
            ty: TypedDataType::String,
        }],
    }];
    CompatibilitySnapshot {
        schema_version: 4,
        manifest_schema: "musubi/config-v4".into(),
        manifest_sha256: musubi_core::Sha256Digest::new([7; 32]),
        generator: crate::test_generator::info(),
        runtime: crate::RuntimeSource::Path {
            root: "../..".into(),
        },
        generator_abi_version: 2,
        runtime_abi_version: 2,
        package_name: "typed-sdk".into(),
        package_version: "0.1.0".into(),
        contracts: Vec::new(),
        typed_data: vec![TypedDataSnapshot {
            id: "messages".into(),
            rust_module: "messages".into(),
            root_evidence: vec![evidence(&types, "Message")],
            types,
            roots: vec![TypedDataRoot::StructHash {
                r#type: "Message".into(),
            }],
            schema_sha256: FixedBytes::new([1; 32]),
            vectors_sha256: FixedBytes::new([2; 32]),
            evidence: TypedDataExternalEvidence {
                solidity_source_sha256: FixedBytes::new([3; 32]),
                artifact_target: "test/Hash.t.sol:HashTest".into(),
                artifact_sha256: FixedBytes::new([4; 32]),
                compiler_version: "solc 0.8.26".into(),
                compiler_settings_sha256: FixedBytes::new([5; 32]),
                vector_command_sha256: FixedBytes::new([6; 32]),
            },
        }],
    }
}

fn evidence(types: &[TypedDataStruct], primary: &str) -> TypedDataRootEvidence {
    TypedDataRootEvidence {
        primary_type: primary.into(),
        type_string: musubi_runtime::typed_data_type_string(types, primary).unwrap(),
        type_hash: musubi_runtime::typed_data_type_hash(types, primary).unwrap(),
        domain_type: None,
        domain_type_string: None,
        domain_type_hash: None,
    }
}

fn has(report: &crate::CompatibilityReport, category: CompatibilityCategory) -> bool {
    report
        .changes
        .iter()
        .any(|change| change.category == category)
}

#[test]
fn typed_data_additive_breaking_and_provenance_are_independent() {
    let old = snapshot();
    let mut provenance = old.clone();
    provenance.typed_data[0].vectors_sha256 = FixedBytes::new([9; 32]);
    let report = diff_snapshots(&old, &provenance);
    assert!(has(&report, CompatibilityCategory::ProvenanceOnly));
    assert!(!has(&report, CompatibilityCategory::TypedDataBreaking));

    let mut additive = old.clone();
    let added = TypedDataStruct {
        name: "Counter".into(),
        fields: vec![TypedDataField {
            name: "value".into(),
            ty: TypedDataType::Uint(32),
        }],
    };
    additive.typed_data[0]
        .root_evidence
        .push(evidence(std::slice::from_ref(&added), "Counter"));
    additive.typed_data[0].types.push(added);
    additive.typed_data[0]
        .roots
        .push(TypedDataRoot::StructHash {
            r#type: "Counter".into(),
        });
    let report = diff_snapshots(&old, &additive);
    assert!(has(&report, CompatibilityCategory::TypedDataAdditive));

    let mut breaking = old.clone();
    breaking.typed_data[0].types[0].fields[0].name = "text".into();
    let report = diff_snapshots(&old, &breaking);
    assert!(has(&report, CompatibilityCategory::TypedDataBreaking));
    assert!(has(&report, CompatibilityCategory::RustSourceBreaking));
}

#[test]
fn adding_a_same_primary_root_reports_the_digest_method_break() {
    let mut old = snapshot();
    let domain = TypedDataStruct {
        name: "EIP712Domain".into(),
        fields: vec![TypedDataField {
            name: "chainId".into(),
            ty: TypedDataType::Uint(256),
        }],
    };
    old.typed_data[0].types.push(domain);
    old.typed_data[0]
        .types
        .sort_by(|left, right| left.name.cmp(&right.name));
    old.typed_data[0].roots[0] = TypedDataRoot::TypedData {
        r#type: "Message".into(),
        domain: "EIP712Domain".into(),
    };
    let domain_type_string =
        musubi_runtime::typed_data_type_string(&old.typed_data[0].types, "EIP712Domain").unwrap();
    let domain_type_hash =
        musubi_runtime::typed_data_type_hash(&old.typed_data[0].types, "EIP712Domain").unwrap();
    old.typed_data[0].root_evidence[0].domain_type = Some("EIP712Domain".into());
    old.typed_data[0].root_evidence[0].domain_type_string = Some(domain_type_string);
    old.typed_data[0].root_evidence[0].domain_type_hash = Some(domain_type_hash);

    let mut new = old.clone();
    new.typed_data[0].roots.insert(
        0,
        TypedDataRoot::StructHash {
            r#type: "Message".into(),
        },
    );
    let additional_evidence = evidence(&new.typed_data[0].types, "Message");
    new.typed_data[0]
        .root_evidence
        .insert(0, additional_evidence);
    let report = diff_snapshots(&old, &new);
    assert!(has(&report, CompatibilityCategory::TypedDataAdditive));
    assert!(has(&report, CompatibilityCategory::RustSourceBreaking));
    assert!(!has(&report, CompatibilityCategory::RustSourceAdditive));
}
