//! Public generation IR mutation and derived-name rendering regressions.

use std::{fs, path::Path};

use musubi_core::{FixedBytes, TypedDataField, TypedDataRoot, TypedDataStruct, TypedDataType};

use crate::{
    Diagnostics, GenerationIr, TypedDataExternalEvidence, TypedDataIr, TypedDataRootEvidence,
    build_ir, load_inputs, render,
};

fn root_evidence(types: &[TypedDataStruct], primary: &str, domain: &str) -> TypedDataRootEvidence {
    TypedDataRootEvidence {
        primary_type: primary.into(),
        type_string: musubi_runtime::typed_data_type_string(types, primary).unwrap(),
        type_hash: musubi_runtime::typed_data_type_hash(types, primary).unwrap(),
        domain_type: Some(domain.into()),
        domain_type_string: Some(musubi_runtime::typed_data_type_string(types, domain).unwrap()),
        domain_type_hash: Some(musubi_runtime::typed_data_type_hash(types, domain).unwrap()),
    }
}

fn typed_ir() -> GenerationIr {
    let types = vec![
        TypedDataStruct {
            name: "Message".into(),
            fields: vec![TypedDataField {
                name: "value".into(),
                ty: TypedDataType::Uint(256),
            }],
        },
        TypedDataStruct {
            name: "OtherDomain".into(),
            fields: vec![TypedDataField {
                name: "chainId".into(),
                ty: TypedDataType::Uint(256),
            }],
        },
        TypedDataStruct {
            name: "type".into(),
            fields: vec![TypedDataField {
                name: "salt".into(),
                ty: TypedDataType::FixedBytes(32),
            }],
        },
    ];
    let roots = vec![
        TypedDataRoot::TypedData {
            r#type: "Message".into(),
            domain: "OtherDomain".into(),
        },
        TypedDataRoot::TypedData {
            r#type: "Message".into(),
            domain: "type".into(),
        },
    ];
    let root_evidence = roots
        .iter()
        .map(|root| root_evidence(&types, root.primary_type(), root.domain_type().unwrap()))
        .collect();
    GenerationIr {
        runtime: crate::RuntimeSource::Path {
            root: "../..".into(),
        },
        manifest_sha256: FixedBytes::new([7; 32]),
        package_name: "typed-render-test".into(),
        package_version: "0.1.0".into(),
        contracts: Vec::new(),
        typed_data: vec![TypedDataIr {
            id: "message".into(),
            module: "message".into(),
            types,
            roots,
            root_evidence,
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
        diagnostics: Diagnostics::new(),
        limits: crate::HardLimits::V1,
    }
}

fn refresh_evidence(ir: &mut GenerationIr) {
    let types = ir.typed_data[0].types.clone();
    let roots = ir.typed_data[0].roots.clone();
    ir.typed_data[0].root_evidence = roots
        .iter()
        .map(|root| root_evidence(&types, root.primary_type(), root.domain_type().unwrap()))
        .collect();
}

fn shared_dag_typed_ir() -> TypedDataIr {
    let mut types = vec![TypedDataStruct {
        name: "Hub".into(),
        fields: (0..128)
            .map(|index| TypedDataField {
                name: format!("value{index}"),
                ty: TypedDataType::Uint(256),
            })
            .collect(),
    }];
    types.extend((0..128).map(|index| TypedDataStruct {
        name: format!("Top{index:03}"),
        fields: vec![TypedDataField {
            name: "hub".into(),
            ty: TypedDataType::Struct("Hub".into()),
        }],
    }));
    let roots = (0..128)
        .map(|index| TypedDataRoot::StructHash {
            r#type: format!("Top{index:03}"),
        })
        .collect::<Vec<_>>();
    let root_evidence = roots
        .iter()
        .map(|root| TypedDataRootEvidence {
            primary_type: root.primary_type().into(),
            type_string: musubi_runtime::typed_data_type_string(&types, root.primary_type())
                .unwrap(),
            type_hash: musubi_runtime::typed_data_type_hash(&types, root.primary_type()).unwrap(),
            domain_type: None,
            domain_type_string: None,
            domain_type_hash: None,
        })
        .collect();
    let mut authority = typed_ir().typed_data.remove(0);
    authority.types = types;
    authority.roots = roots;
    authority.root_evidence = root_evidence;
    authority
}

fn deployable_ir(root: &Path) -> GenerationIr {
    fs::write(
        root.join("abi.json"),
        br#"[{"type":"function","name":"owner","stateMutability":"view","inputs":[],"outputs":[{"name":"owner","type":"address"}]}]"#,
    )
    .unwrap();
    fs::write(root.join("creation.json"), br#""0x60006000f3""#).unwrap();
    fs::write(
        root.join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[generation]
output = "sdk"

[package]
name = "render-test"
version = "0.1.0"
edition = "2024"

[[contracts]]
kind = "deployable"
id = "vault"
abi = "abi.json"
creation_bytecode = "creation.json"
"#,
    )
    .unwrap();
    build_ir(load_inputs(&root.join("musubi.toml")).unwrap()).unwrap()
}

#[test]
fn public_ir_mutations_fail_before_rendering() {
    let mut typed = typed_ir();
    typed.typed_data[0].types.clear();
    assert!(render(&typed, &crate::test_generator::info()).is_err());

    let root = tempfile::tempdir().unwrap();
    let mut deployable = deployable_ir(root.path());
    deployable.contracts[0].creation = None;
    assert!(render(&deployable, &crate::test_generator::info()).is_err());

    let mut abi_drift = deployable_ir(root.path());
    abi_drift.contracts[0].abi.functions.clear();
    assert!(render(&abi_drift, &crate::test_generator::info()).is_err());
}

#[test]
fn public_ir_abi_structure_reports_resource_limits_before_rendering() {
    let root = tempfile::tempdir().unwrap();
    for bytes in ["[0,0]", r#"[{"inputs":[0],"outputs":[0]}]"#] {
        let mut ir = deployable_ir(root.path());
        ir.limits.abi_items = 1;
        ir.limits.abi_components = 1;
        ir.contracts[0].canonical_abi = bytes.as_bytes().to_vec();
        assert_eq!(
            render(&ir, &crate::test_generator::info())
                .unwrap_err()
                .diagnostic
                .code,
            crate::DiagnosticCode::LimitExceeded
        );
        assert!(!root.path().join("sdk").exists());
    }
}

#[test]
fn public_ir_obeys_lowered_identifier_nesting_and_bytecode_limits() {
    let mut identifier = typed_ir();
    identifier.limits.identifier_bytes = 10;
    assert!(render(&identifier, &crate::test_generator::info()).is_err());

    let mut nesting = typed_ir();
    nesting.typed_data[0].types[0].fields[0].ty = TypedDataType::Array(Box::new(
        TypedDataType::Array(Box::new(TypedDataType::Uint(256))),
    ));
    refresh_evidence(&mut nesting);
    nesting.limits.nesting = 1;
    assert!(render(&nesting, &crate::test_generator::info()).is_err());

    let mut fixed = typed_ir();
    fixed.typed_data[0].types[0].fields[0].ty = TypedDataType::FixedArray(
        Box::new(TypedDataType::FixedArray(
            Box::new(TypedDataType::Uint(256)),
            2,
        )),
        2,
    );
    refresh_evidence(&mut fixed);
    fixed.limits.typed_data_fixed_array_elements = 3;
    assert!(render(&fixed, &crate::test_generator::info()).is_err());

    let root = tempfile::tempdir().unwrap();
    let mut bytecode = deployable_ir(root.path());
    bytecode.limits.bytecode_bytes = 4;
    assert!(render(&bytecode, &crate::test_generator::info()).is_err());
}

#[test]
fn public_ir_obeys_lowered_contract_identity_limits() {
    let root = tempfile::tempdir().unwrap();
    let mut id = deployable_ir(root.path());
    id.contracts[0].id = "vaults".into();
    assert!(render(&id, &crate::test_generator::info()).is_ok());
    id.limits.identifier_bytes = 5;
    assert!(render(&id, &crate::test_generator::info()).is_err());

    let mut module = deployable_ir(root.path());
    module.contracts[0].module = "vaults".into();
    assert!(render(&module, &crate::test_generator::info()).is_ok());
    module.limits.identifier_bytes = 5;
    assert!(render(&module, &crate::test_generator::info()).is_err());
}

#[test]
fn public_ir_rejects_non_cargo_package_identity() {
    for (name, version) in [("not a package", "0.1.0"), ("typed-sdk", "version one")] {
        let mut ir = typed_ir();
        ir.package_name = name.into();
        ir.package_version = version.into();
        assert!(render(&ir, &crate::test_generator::info()).is_err());
    }
}

#[test]
fn keyword_domain_suffix_is_valid_inside_a_derived_method_name() {
    let tree = render(&typed_ir(), &crate::test_generator::info()).unwrap();
    let source = crate::output_test_support::rust_sources(&tree, "src/message/types");
    assert!(source.contains("pub fn digest_with_type("));
    assert!(!source.contains("digest_with_r#type"));
}

#[test]
fn shared_dag_render_stops_at_the_output_budget() {
    let typed = shared_dag_typed_ir();
    let mut tree = crate::GeneratedTree::with_limits(crate::HardLimits {
        generated_bytes: 64 * 1024,
        ..crate::HardLimits::V1
    });
    assert!(crate::render_typed_data::typed_data_tree(&mut tree, &typed).is_err());
}

#[test]
fn large_nested_abi_renderers_stop_at_the_output_budget() {
    let root = tempfile::tempdir().unwrap();
    let _ = deployable_ir(root.path());
    let components = (0..128)
        .map(|index| {
            serde_json::json!({
                "name": format!("value{index}"),
                "type": "uint256"
            })
        })
        .collect::<Vec<_>>();
    let abi = serde_json::json!([{
        "type": "function",
        "name": "store",
        "stateMutability": "nonpayable",
        "inputs": [{"name": "record", "type": "tuple", "components": components}],
        "outputs": []
    }]);
    fs::write(
        root.path().join("abi.json"),
        serde_json::to_vec(&abi).unwrap(),
    )
    .unwrap();
    let mut ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    let contract = &ir.contracts[0];
    assert!(
        crate::render_abi::function(
            &mut crate::render_source::BoundedSource::new(1024),
            &contract.abi.functions[0]
        )
        .is_err()
    );
    assert!(
        crate::render_owned_tuple::render_owned_tuple(
            &mut crate::render_source::BoundedSource::new(1024),
            contract.owned_types.tuples().next().unwrap()
        )
        .is_err()
    );
    let layout = crate::render_contract_layout::ContractLayout::new(contract).unwrap();
    assert!(crate::render_contract::contract_source(contract, &layout, 1024).is_err());

    ir.limits.generated_bytes = 1024;
    assert!(render(&ir, &crate::test_generator::info()).is_err());
}
