//! Generated-crate acceptance tests for owned tuple identities and deterministic traversal.

use crate::output_test_support::{CargoValidation, generate};

use std::{fs, path::Path};

use serde_json::json;

use crate::{build_ir, load_inputs, render};

#[test]
fn same_wire_different_surface_compiles_without_ast() {
    let root = target_tempdir();
    write_fixture(root.path(), &decision_abi(), None);
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let source = generated_types(&tree);
    assert_eq!(source.matches("pub struct Tuple").count(), 2);
    assert!(source.contains("pub should_place: bool"));
    assert!(source.contains("pub should_rebalance: bool"));
    generate(&tree, &root.path().join("sdk"), CargoValidation::Required).unwrap();
}

#[test]
fn verified_enrichment_and_different_fallback_surfaces_compile_with_ast() {
    let root = target_tempdir();
    let mut abi = decision_abi();
    abi.push(json!({
        "type":"function", "name":"verified", "stateMutability":"view", "inputs":[],
        "outputs":[{"name":"record","type":"tuple","internalType":"struct Vault.Record",
            "components":[{"name":"amount","type":"uint256"}]}]
    }));
    let ast = json!({"nodeType":"SourceUnit","nodes":[{
        "nodeType":"StructDefinition", "name":"Record", "canonicalName":"Vault.Record",
        "members":[{"nodeType":"VariableDeclaration", "name":"amount",
            "typeDescriptions":{"typeString":"uint256"}}]
    }]});
    write_fixture(root.path(), &abi, Some(&ast));
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    assert_eq!(ir.contracts[0].enrichments.len(), 1);
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let source = generated_types(&tree);
    assert_eq!(source.matches("pub struct Tuple").count(), 2);
    assert!(source.contains("pub struct Record"));
    generate(&tree, &root.path().join("sdk"), CargoValidation::Required).unwrap();
}

#[test]
fn abi_item_order_preserves_owned_names_and_generated_rust() {
    let first = target_tempdir();
    let second = target_tempdir();
    let mut reversed = decision_abi();
    reversed.reverse();
    write_fixture(first.path(), &decision_abi(), None);
    write_fixture(second.path(), &reversed, None);

    let first_ir = build_ir(load_inputs(&first.path().join("musubi.toml")).unwrap()).unwrap();
    let second_ir = build_ir(load_inputs(&second.path().join("musubi.toml")).unwrap()).unwrap();
    assert_ne!(
        first_ir.contracts[0].raw_abi_sha256,
        second_ir.contracts[0].raw_abi_sha256
    );
    assert_eq!(
        first_ir.contracts[0].canonical_abi,
        second_ir.contracts[0].canonical_abi
    );
    let first_tree = render(&first_ir, &crate::test_generator::info()).unwrap();
    let second_tree = render(&second_ir, &crate::test_generator::info()).unwrap();
    assert_eq!(generated_types(&first_tree), generated_types(&second_tree));
    assert_eq!(generated_abi(&first_tree), generated_abi(&second_tree));
    assert_ne!(
        generated_contract(&first_tree),
        generated_contract(&second_tree)
    );
}

fn decision_abi() -> Vec<serde_json::Value> {
    vec![
        decision_function(
            "initial",
            "shouldPlace",
            "struct LpvTypes.InitialPlacementDecision",
        ),
        decision_function(
            "rebalance",
            "shouldRebalance",
            "struct LpvTypes.RebalanceDecision",
        ),
    ]
}

fn decision_function(name: &str, flag: &str, internal: &str) -> serde_json::Value {
    json!({
        "type":"function", "name":name, "stateMutability":"view", "inputs":[],
        "outputs":[{"name":"decision","type":"tuple","internalType":internal,"components":[
            {"name":flag,"type":"bool"},
            {"name":"targetCenterTick","type":"int24"},
            {"name":"targetRangeWidth","type":"int24"}
        ]}]
    })
}

fn target_tempdir() -> tempfile::TempDir {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    fs::create_dir_all(workspace.join("target")).unwrap();
    tempfile::tempdir_in(workspace.join("target")).unwrap()
}

fn write_fixture(root: &Path, abi: &[serde_json::Value], ast: Option<&serde_json::Value>) {
    fs::write(root.join("abi.json"), serde_json::to_vec(abi).unwrap()).unwrap();
    fs::write(root.join("creation.json"), br#""0x60006000f3""#).unwrap();
    if let Some(ast) = ast {
        fs::write(root.join("ast.json"), serde_json::to_vec(ast).unwrap()).unwrap();
    }
    let ast_line = ast.map_or("", |_| "ast = \"ast.json\"\n");
    fs::write(
        root.join("musubi.toml"),
        format!(
            "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n\n[generation]\noutput = \"sdk\"\n\n[package]\nname = \"owned-tuple-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[contracts]]\nkind = \"deployable\"\nid = \"planner\"\nabi = \"abi.json\"\ncreation_bytecode = \"creation.json\"\n{ast_line}"
        ),
    )
    .unwrap();
}

fn generated_types(tree: &crate::GeneratedTree) -> String {
    crate::output_test_support::rust_sources(tree, "src/planner/shared_types")
}

fn generated_contract(tree: &crate::GeneratedTree) -> String {
    generated_file(tree, "src/planner/artifact.rs")
}

fn generated_abi(tree: &crate::GeneratedTree) -> String {
    crate::output_test_support::rust_sources(tree, "src/planner/operations")
}

fn generated_file(tree: &crate::GeneratedTree, path: &str) -> String {
    let file = tree
        .files()
        .find(|file| file.path == Path::new(path))
        .unwrap();
    String::from_utf8(file.contents).unwrap()
}
