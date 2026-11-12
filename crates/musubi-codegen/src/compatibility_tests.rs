//! Compatibility category and strict snapshot schema tests.

use musubi_core::{
    AbiField, AbiType, BytecodeRange, ContractAbi, ExternalArtifactCoherence, FixedBytes,
    FunctionDescriptor, ImmutableReference, LinkReference, StateMutability,
};

use crate::{
    CompatibilityCategory as Category, CompatibilitySnapshot, ContractSnapshot, DeploymentSnapshot,
    Enrichment, EnrichmentKind, ProvenanceSnapshot, diff_snapshots, parse_snapshot, snapshot_json,
};

#[test]
fn additive_overload_and_breaking_wire_changes_are_distinct() {
    let old = snapshot();
    let mut additive = old.clone();
    additive.contracts[0].abi.functions.push(function(
        "bar",
        "bar(address)",
        [0x64, 0x67, 0xea, 0x56],
        AbiType::Address,
    ));
    let report = diff_snapshots(&old, &additive);
    assert!(has(&report, Category::EvmWireAdditive));
    assert!(has(&report, Category::RustSourceAdditive));

    let mut overload = old.clone();
    overload.contracts[0].abi.functions.push(function(
        "foo",
        "foo(address)",
        [0xfd, 0xf8, 0x0b, 0xda],
        AbiType::Address,
    ));
    let report = diff_snapshots(&old, &overload);
    assert!(has(&report, Category::EvmWireAdditive));
    assert!(has(&report, Category::RustSourceBreaking));
    assert!(!has(&report, Category::RustSourceAdditive));

    let mut breaking = old.clone();
    breaking.contracts[0].abi.functions[0] = function(
        "foo",
        "foo(address)",
        [0xfd, 0xf8, 0x0b, 0xda],
        AbiType::Address,
    );
    let report = diff_snapshots(&old, &breaking);
    assert!(has(&report, Category::EvmWireBreaking));
    assert!(has(&report, Category::RustSourceBreaking));
}

#[test]
fn rust_deployment_and_provenance_changes_are_independent() {
    let old = snapshot();
    let mut renamed = old.clone();
    renamed.contracts[0].abi.functions[0].inputs[0].name = "renamed".into();
    let report = diff_snapshots(&old, &renamed);
    assert!(has(&report, Category::RustSourceBreaking));
    assert!(!has(&report, Category::EvmWireBreaking));

    let mut deployment = old.clone();
    deployment.contracts[0]
        .deployment
        .as_mut()
        .unwrap()
        .creation_template_keccak256 = FixedBytes::new([9; 32]);
    let report = diff_snapshots(&old, &deployment);
    assert_eq!(report.changes.len(), 1);
    assert!(has(&report, Category::DeploymentIdentityChanged));

    let mut provenance = old.clone();
    provenance.contracts[0].provenance.raw_abi_sha256 = FixedBytes::new([8; 32]);
    let report = diff_snapshots(&old, &provenance);
    assert_eq!(report.changes.len(), 1);
    assert!(has(&report, Category::ProvenanceOnly));
}

#[test]
fn adding_or_removing_an_interface_has_no_deployment_change() {
    let mut interface = snapshot().contracts.remove(0);
    interface.id = "Interface".into();
    interface.rust_module = "interface_contract".into();
    interface.kind = musubi_core::ArtifactKind::Interface;
    interface.deployment = None;
    let mut empty = snapshot();
    empty.contracts.clear();
    let mut present = empty.clone();
    present.contracts.push(interface);

    for report in [
        diff_snapshots(&empty, &present),
        diff_snapshots(&present, &empty),
    ] {
        assert!(!has(&report, Category::DeploymentIdentityChanged));
    }
}

#[test]
fn owned_tuple_field_rename_is_rust_breaking_but_wire_unchanged() {
    let old = snapshot();
    let mut renamed = old.clone();
    let tuple = AbiType::Tuple(vec![
        AbiField {
            name: "shouldPlace".into(),
            ty: AbiType::Bool,
            internal_type: None,
        },
        AbiField {
            name: "targetCenterTick".into(),
            ty: AbiType::Int(24),
            internal_type: None,
        },
    ]);
    old_tuple_input(&mut renamed, tuple.clone());
    let mut old = old;
    old_tuple_input(&mut old, tuple);
    let AbiType::Tuple(fields) = &mut renamed.contracts[0].abi.functions[0].inputs[0].ty else {
        panic!("test fixture must contain a tuple");
    };
    fields[0].name = "shouldRebalance".into();

    let report = diff_snapshots(&old, &renamed);
    assert!(has(&report, Category::RustSourceBreaking));
    assert!(!has(&report, Category::EvmWireBreaking));
    assert!(!has(&report, Category::EvmWireAdditive));
}

#[test]
fn unresolved_internal_type_change_is_provenance_only() {
    let old = snapshot();
    let mut changed = old.clone();
    changed.contracts[0].abi.functions[0].inputs[0].internal_type =
        Some("type Missing.Amount".into());
    changed.contracts[0].provenance.raw_abi_sha256 = FixedBytes::new([8; 32]);

    let report = diff_snapshots(&old, &changed);
    assert_eq!(report.changes.len(), 1);
    assert!(has(&report, Category::ProvenanceOnly));
    assert!(!has(&report, Category::RustSourceBreaking));
    assert!(!has(&report, Category::EvmWireBreaking));
}

#[test]
fn unresolved_internal_type_inside_verified_struct_is_provenance_only() {
    let mut old = snapshot();
    let fields = vec![AbiField {
        name: "value".into(),
        ty: AbiType::Uint(256),
        internal_type: Some("uint256".into()),
    }];
    old.contracts[0].abi.functions[0].inputs[0] = AbiField {
        name: "record".into(),
        ty: AbiType::Tuple(fields.clone()),
        internal_type: Some("struct Vault.Record".into()),
    };
    old.contracts[0].abi.functions[0].signature = "foo((uint256))".into();
    old.contracts[0].enrichments = vec![Enrichment {
        internal_type: "struct Vault.Record".into(),
        rust_name: "Record".into(),
        kind: EnrichmentKind::Struct { fields },
    }];
    let mut changed = old.clone();
    let AbiType::Tuple(abi_fields) = &mut changed.contracts[0].abi.functions[0].inputs[0].ty else {
        panic!("test fixture must contain a tuple");
    };
    abi_fields[0].internal_type = None;
    let EnrichmentKind::Struct { fields } = &mut changed.contracts[0].enrichments[0].kind else {
        panic!("test fixture must contain a struct enrichment");
    };
    fields[0].internal_type = None;
    changed.contracts[0].provenance.raw_abi_sha256 = FixedBytes::new([8; 32]);

    let report = diff_snapshots(&old, &changed);
    assert_eq!(report.changes.len(), 1);
    assert!(has(&report, Category::ProvenanceOnly));
    assert!(!has(&report, Category::RustSourceBreaking));
    assert!(!has(&report, Category::EvmWireBreaking));
}

#[test]
fn snapshot_json_is_canonical_and_closed() {
    let snapshot = snapshot();
    let bytes = snapshot_json(&snapshot).unwrap();
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(parse_snapshot(&bytes).unwrap(), snapshot);
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for pointer in [
        "",
        "/contracts/0/abi/functions/0",
        "/contracts/0/abi/functions/0/inputs/0",
        "/contracts/0/abi/functions/0/inputs/0/ty",
        "/contracts/0/deployment/creation_links/0",
        "/contracts/0/deployment/creation_links/0/ranges/0",
        "/contracts/0/deployment/runtime",
        "/contracts/0/deployment/runtime/immutables/0",
        "/contracts/0/provenance",
    ] {
        let mut mutated = document.clone();
        mutated
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), serde_json::Value::Bool(true));
        assert!(
            parse_snapshot(&serde_json::to_vec(&mutated).unwrap()).is_err(),
            "unknown field accepted at {pointer}"
        );
    }
}

fn snapshot() -> CompatibilitySnapshot {
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
        package_name: "fixture-sdk".into(),
        package_version: "1.0.0".into(),
        contracts: vec![ContractSnapshot {
            id: "Vault".into(),
            rust_module: "vault".into(),
            kind: musubi_core::ArtifactKind::Deployable,
            abi: ContractAbi {
                functions: vec![function(
                    "foo",
                    "foo(uint256)",
                    [0x2f, 0xbe, 0xbd, 0x38],
                    AbiType::Uint(256),
                )],
                ..ContractAbi::default()
            },
            enrichments: Vec::new(),
            deployment: Some(DeploymentSnapshot {
                creation_template_keccak256: FixedBytes::new([1; 32]),
                creation_template_bytes: 21,
                creation_links: vec![LinkReference {
                    source: "Library.sol".into(),
                    library: "Library".into(),
                    ranges: vec![BytecodeRange {
                        start: 1,
                        length: 20,
                    }],
                }],
                runtime: Some(crate::RuntimeDeploymentSnapshot {
                    template_keccak256: FixedBytes::new([4; 32]),
                    template_bytes: 32,
                    links: Vec::new(),
                    immutables: vec![ImmutableReference {
                        id: "0".into(),
                        ranges: vec![BytecodeRange {
                            start: 0,
                            length: 32,
                        }],
                    }],
                }),
            }),
            provenance: ProvenanceSnapshot {
                canonical_abi_sha256: FixedBytes::new([2; 32]),
                raw_abi_sha256: FixedBytes::new([3; 32]),
                ast_sha256: None,
                metadata_sha256: None,
                storage_layout_sha256: None,
                external_artifact_coherence: ExternalArtifactCoherence::Unverified,
            },
        }],
        typed_data: Vec::new(),
    }
}

fn function(name: &str, signature: &str, selector: [u8; 4], input: AbiType) -> FunctionDescriptor {
    FunctionDescriptor {
        name: name.into(),
        signature: signature.into(),
        selector,
        inputs: vec![AbiField {
            name: "value".into(),
            ty: input,
            internal_type: None,
        }],
        outputs: Vec::new(),
        state_mutability: StateMutability::Nonpayable,
    }
}

fn old_tuple_input(snapshot: &mut CompatibilitySnapshot, tuple: AbiType) {
    let function = &mut snapshot.contracts[0].abi.functions[0];
    function.signature = "foo((bool,int24))".into();
    function.selector = [0x7b, 0x87, 0x6e, 0x33];
    function.inputs[0].ty = tuple;
}

fn has(report: &crate::CompatibilityReport, category: Category) -> bool {
    report
        .changes
        .iter()
        .any(|change| change.category == category)
}
