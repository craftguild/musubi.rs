//! Fail-closed semantic checks for untrusted compatibility snapshots.

use std::fs;

use musubi_core::{
    AbiField, AbiType, ArtifactKind, BytecodeRange, ConstructorDescriptor, ContractAbi,
    EventDescriptor, EventField, ExternalArtifactCoherence, FixedBytes, FunctionDescriptor,
    LinkReference, StateMutability, TypedDataField, TypedDataRoot, TypedDataStruct, TypedDataType,
    keccak256,
};

use crate::{
    CompatibilitySnapshot, ContractSnapshot, DeploymentSnapshot, Enrichment, EnrichmentKind,
    HardLimits, ProvenanceSnapshot, TypedDataExternalEvidence, TypedDataRootEvidence,
    TypedDataSnapshot, build_ir, build_snapshot, load_inputs, parse_snapshot, snapshot_json,
};

#[test]
fn snapshot_rejects_impossible_capabilities_and_duplicate_ids() {
    let valid = contract_snapshot();
    assert!(parse(&valid).is_ok());

    let mut invalid_generator = valid.clone();
    invalid_generator.generator.version = "not-semver".into();
    assert!(parse(&invalid_generator).is_err());

    let mut deployable_without_bytecode = valid.clone();
    deployable_without_bytecode.contracts[0].kind = ArtifactKind::Deployable;
    assert!(parse(&deployable_without_bytecode).is_err());

    let mut interface_with_bytecode = valid.clone();
    interface_with_bytecode.contracts[0].deployment = Some(DeploymentSnapshot {
        creation_template_keccak256: FixedBytes::new([1; 32]),
        creation_template_bytes: 1,
        creation_links: Vec::new(),
        runtime: None,
    });
    assert!(parse(&interface_with_bytecode).is_err());

    let mut interface_with_storage = valid.clone();
    interface_with_storage.contracts[0]
        .provenance
        .storage_layout_sha256 = Some(FixedBytes::new([9; 32]));
    assert!(parse(&interface_with_storage).is_err());

    let mut duplicate = valid;
    duplicate.contracts.push(duplicate.contracts[0].clone());
    assert!(parse(&duplicate).is_err());
}

#[test]
fn snapshot_rejects_noncanonical_typed_hash_text() {
    let valid = serde_json::to_value(typed_snapshot()).unwrap();
    for (pointer, replacement) in [
        ("/typed_data/0/schema_sha256", "03".repeat(32)),
        (
            "/typed_data/0/root_evidence/0/type_hash",
            format!("0x{}", "AA".repeat(32)),
        ),
        (
            "/typed_data/0/evidence/vector_command_sha256",
            format!("0x{}", "00".repeat(32)),
        ),
    ] {
        let mut mutated = valid.clone();
        *mutated.pointer_mut(pointer).unwrap() = serde_json::Value::String(replacement);
        assert!(parse_snapshot(&serde_json::to_vec(&mutated).unwrap()).is_err());
    }
}

#[test]
fn snapshot_rejects_noncanonical_contract_hash_text() {
    let valid = serde_json::to_value(contract_snapshot()).unwrap();
    for (pointer, replacement) in [
        (
            "/contracts/0/provenance/canonical_abi_sha256",
            "11".repeat(32),
        ),
        (
            "/contracts/0/provenance/raw_abi_sha256",
            format!("0x{}", "AA".repeat(32)),
        ),
    ] {
        let mut mutated = valid.clone();
        *mutated.pointer_mut(pointer).unwrap() = serde_json::Value::String(replacement);
        assert!(parse_snapshot(&serde_json::to_vec(&mutated).unwrap()).is_err());
    }
}

#[test]
fn snapshot_rejects_duplicate_json_fields_before_lexical_inspection() {
    let bytes = snapshot_json(&typed_snapshot()).unwrap();
    let text = String::from_utf8(bytes).unwrap().replacen(
        "\"schema_version\": 4,",
        "\"schema_version\": 4,\n  \"schema_version\": 4,",
        1,
    );
    assert!(parse_snapshot(text.as_bytes()).is_err());
}

#[test]
fn snapshot_rejects_typed_graph_and_root_evidence_drift() {
    let valid = typed_snapshot();
    assert!(parse(&valid).is_ok());

    let mut cyclic = valid.clone();
    cyclic.typed_data[0].types[0].fields[0].ty = TypedDataType::Struct("Message".into());
    assert!(parse(&cyclic).is_err());

    let mut tampered = valid;
    tampered.typed_data[0].root_evidence[0].type_string = "Message(uint256 other)".into();
    assert!(parse(&tampered).is_err());

    let mut absolute_target = typed_snapshot();
    absolute_target.typed_data[0].evidence.artifact_target = "/private/out.json:Message".into();
    assert!(parse(&absolute_target).is_err());

    let mut unsafe_compiler = typed_snapshot();
    unsafe_compiler.typed_data[0].evidence.compiler_version = "/private/solc\nsecret".into();
    assert!(parse(&unsafe_compiler).is_err());

    let mut reserved = typed_snapshot();
    reserved.typed_data[0].types[0].name = "Box".into();
    reserved.typed_data[0].roots[0] = TypedDataRoot::StructHash {
        r#type: "Box".into(),
    };
    reserved.typed_data[0].root_evidence[0] = TypedDataRootEvidence {
        primary_type: "Box".into(),
        type_string: musubi_runtime::typed_data_type_string(&reserved.typed_data[0].types, "Box")
            .unwrap(),
        type_hash: musubi_runtime::typed_data_type_hash(&reserved.typed_data[0].types, "Box")
            .unwrap(),
        domain_type: None,
        domain_type_string: None,
        domain_type_hash: None,
    };
    assert!(parse(&reserved).is_err());
}

#[test]
fn snapshot_rejects_forged_enum_event_and_constructor_semantics() {
    let mut oversized_variant = contract_snapshot();
    let signature = "mode(uint8)";
    oversized_variant.contracts[0]
        .abi
        .functions
        .push(FunctionDescriptor {
            name: "mode".into(),
            signature: signature.into(),
            selector: keccak256(signature.as_bytes()).as_ref()[..4]
                .try_into()
                .unwrap(),
            inputs: vec![AbiField {
                name: "value".into(),
                ty: AbiType::Uint(8),
                internal_type: Some("enum V.Mode".into()),
            }],
            outputs: Vec::new(),
            state_mutability: StateMutability::View,
        });
    oversized_variant.contracts[0].enrichments.push(Enrichment {
        internal_type: "enum V.Mode".into(),
        rust_name: "Mode".into(),
        kind: EnrichmentKind::Enum {
            variants: vec!["x".repeat(HardLimits::V1.identifier_bytes + 1)],
            underlying: AbiType::Uint(8),
        },
    });
    assert!(parse(&oversized_variant).is_err());

    let mut invalid_event = contract_snapshot();
    let signature = "TooMany(uint256,uint256,uint256,uint256)";
    invalid_event.contracts[0].abi.events.push(EventDescriptor {
        name: "TooMany".into(),
        signature: signature.into(),
        topic: Some(keccak256(signature.as_bytes()).into_array()),
        inputs: (0..4)
            .map(|index| EventField {
                field: AbiField {
                    name: format!("value{index}"),
                    ty: AbiType::Uint(256),
                    internal_type: None,
                },
                indexed: true,
            })
            .collect(),
        anonymous: false,
    });
    assert!(parse(&invalid_event).is_err());

    let mut invalid_constructor = contract_snapshot();
    invalid_constructor.contracts[0].kind = ArtifactKind::Deployable;
    invalid_constructor.contracts[0].deployment = Some(DeploymentSnapshot {
        creation_template_keccak256: FixedBytes::new([3; 32]),
        creation_template_bytes: 1,
        creation_links: Vec::new(),
        runtime: None,
    });
    invalid_constructor.contracts[0].abi.constructor = Some(ConstructorDescriptor {
        inputs: Vec::new(),
        state_mutability: StateMutability::View,
    });
    assert!(parse(&invalid_constructor).is_err());
}

#[test]
fn snapshot_rejects_forged_provenance_and_bytecode_inventories() {
    let mut zero_hash = contract_snapshot();
    zero_hash.contracts[0].provenance.raw_abi_sha256 = FixedBytes::new([0; 32]);
    assert!(parse(&zero_hash).is_err());

    let mut bad_inventory = contract_snapshot();
    bad_inventory.contracts[0].kind = ArtifactKind::Deployable;
    bad_inventory.contracts[0].deployment = Some(DeploymentSnapshot {
        creation_template_keccak256: FixedBytes::new([3; 32]),
        creation_template_bytes: 20,
        creation_links: vec![LinkReference {
            source: "Library.sol".into(),
            library: "Library".into(),
            ranges: vec![BytecodeRange {
                start: 0,
                length: 19,
            }],
        }],
        runtime: None,
    });
    assert!(parse(&bad_inventory).is_err());

    let mut out_of_bounds = bad_inventory;
    out_of_bounds.contracts[0]
        .deployment
        .as_mut()
        .unwrap()
        .creation_links[0]
        .ranges[0] = BytecodeRange {
        start: 1,
        length: 20,
    };
    assert!(parse(&out_of_bounds).is_err());

    for bytes in [0, HardLimits::V1.bytecode_bytes + 1] {
        let mut invalid_length = out_of_bounds.clone();
        invalid_length.contracts[0]
            .deployment
            .as_mut()
            .unwrap()
            .creation_template_bytes = bytes;
        assert!(parse(&invalid_length).is_err());
    }
}

#[test]
fn unsorted_compiler_ranges_are_canonicalized_before_snapshot_round_trip() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("abi.json"), b"[]").unwrap();
    fs::write(
        root.path().join("creation.json"),
        format!(
            r#"{{"object":"0x{}","linkReferences":{{"L.sol":{{"L":[{{"start":21,"length":20}},{{"start":0,"length":20}}]}}}}}}"#,
            "00".repeat(41)
        ),
    )
    .unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."
[generation]
output = "sdk"

[package]
name = "snapshot-test"
version = "0.1.0"
edition = "2024"
[[contracts]]
kind = "deployable"
id = "Vault"
abi = "abi.json"
creation_bytecode = "creation.json"
"#,
    )
    .unwrap();
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    let snapshot = build_snapshot(&ir, &crate::test_generator::info());
    let parsed = parse_snapshot(&snapshot_json(&snapshot).unwrap()).unwrap();
    let ranges = &parsed.contracts[0]
        .deployment
        .as_ref()
        .unwrap()
        .creation_links[0]
        .ranges;
    assert_eq!((ranges[0].start, ranges[1].start), (0, 21));
    assert_eq!(
        parsed.contracts[0]
            .deployment
            .as_ref()
            .unwrap()
            .creation_template_bytes,
        41
    );
}

pub(crate) fn contract_snapshot() -> CompatibilitySnapshot {
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
        package_name: "interface-sdk".into(),
        package_version: "0.1.0".into(),
        contracts: vec![ContractSnapshot {
            id: "Status".into(),
            rust_module: "status".into(),
            kind: ArtifactKind::Interface,
            abi: ContractAbi::default(),
            enrichments: Vec::new(),
            deployment: None,
            provenance: ProvenanceSnapshot {
                canonical_abi_sha256: FixedBytes::new([1; 32]),
                raw_abi_sha256: FixedBytes::new([2; 32]),
                ast_sha256: None,
                metadata_sha256: None,
                storage_layout_sha256: None,
                external_artifact_coherence: ExternalArtifactCoherence::Unverified,
            },
        }],
        typed_data: Vec::new(),
    }
}

fn typed_snapshot() -> CompatibilitySnapshot {
    let types = vec![TypedDataStruct {
        name: "Message".into(),
        fields: vec![TypedDataField {
            name: "value".into(),
            ty: TypedDataType::Uint(256),
        }],
    }];
    let roots = vec![TypedDataRoot::StructHash {
        r#type: "Message".into(),
    }];
    let root_evidence = vec![TypedDataRootEvidence {
        primary_type: "Message".into(),
        type_string: musubi_runtime::typed_data_type_string(&types, "Message").unwrap(),
        type_hash: musubi_runtime::typed_data_type_hash(&types, "Message").unwrap(),
        domain_type: None,
        domain_type_string: None,
        domain_type_hash: None,
    }];
    let mut snapshot = contract_snapshot();
    snapshot.contracts.clear();
    snapshot.typed_data.push(TypedDataSnapshot {
        id: "messages".into(),
        rust_module: "messages".into(),
        types,
        roots,
        root_evidence,
        schema_sha256: FixedBytes::new([3; 32]),
        vectors_sha256: FixedBytes::new([4; 32]),
        evidence: TypedDataExternalEvidence {
            solidity_source_sha256: FixedBytes::new([5; 32]),
            artifact_target: "test/Message.t.sol:MessageTest".into(),
            artifact_sha256: FixedBytes::new([6; 32]),
            compiler_version: "solc 0.8.26".into(),
            compiler_settings_sha256: FixedBytes::new([7; 32]),
            vector_command_sha256: FixedBytes::new([8; 32]),
        },
    });
    snapshot
}

pub(crate) fn parse(
    snapshot: &CompatibilitySnapshot,
) -> Result<CompatibilitySnapshot, crate::SnapshotError> {
    parse_snapshot(&snapshot_json(snapshot).unwrap())
}
