//! Owned tuple identity encoding, reuse, recursion, and collision tests.

use serde_json::json;

use crate::{
    DiagnosticCode, Diagnostics, HardLimits, abi_parse::parse_abi, abi_type::canonical_type,
    ast::enrich, names::rust_field, owned_identity::OwnedTypeRegistry,
};

#[test]
fn same_wire_different_fields_have_distinct_owned_identities() {
    let abi = decision_abi();
    let initial = &abi.functions[0].outputs[0];
    let rebalance = &abi.functions[1].outputs[0];
    assert_eq!(canonical_type(&initial.ty), "(bool,int24,int24)");
    assert_eq!(canonical_type(&rebalance.ty), "(bool,int24,int24)");

    let registry = OwnedTypeRegistry::build(&abi, &[], HardLimits::V1).unwrap();
    assert_eq!(registry.tuples().count(), 2);
    assert_ne!(
        registry.identity(&initial.ty, initial.internal_type.as_deref()),
        registry.identity(&rebalance.ty, rebalance.internal_type.as_deref())
    );
    let names = registry
        .tuples()
        .map(|tuple| tuple.rust_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "Tuplea366b9f2ea8f039490e83d5a09fefd03",
            "Tuplef8bbbf6550f06e7da4d52a9907c91296",
        ]
    );
    assert_eq!(
        registry.anonymous_name(&initial.ty, None),
        Some("Tuplef8bbbf6550f06e7da4d52a9907c91296")
    );
}

#[test]
fn identical_owned_surface_is_reused() {
    let abi = parse_abi(
        &[
            function("a", &tuple("pair", "value", "amount", "tuple")),
            function("b", &tuple("pair", "value", "amount", "tuple")),
        ],
        HardLimits::V1,
    )
    .unwrap();
    let registry = OwnedTypeRegistry::build(&abi, &[], HardLimits::V1).unwrap();
    assert_eq!(registry.tuples().count(), 1);
}

#[test]
fn nested_tuple_and_array_kinds_are_recursive_identity_inputs() {
    let abi = parse_abi(
        &[
            function("dynamic_a", &outer("children", "shouldPlace", "tuple[]")),
            function(
                "dynamic_b",
                &outer("children", "shouldRebalance", "tuple[]"),
            ),
            function("fixed_a", &outer("children", "shouldPlace", "tuple[2]")),
        ],
        HardLimits::V1,
    )
    .unwrap();
    let registry = OwnedTypeRegistry::build(&abi, &[], HardLimits::V1).unwrap();
    assert_eq!(registry.tuples().count(), 5);
    let identities = abi
        .functions
        .iter()
        .map(|function| {
            let field = &function.outputs[0];
            registry
                .identity(&field.ty, field.internal_type.as_deref())
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_ne!(identities[0], identities[1]);
    assert_ne!(identities[0], identities[2]);
    assert_ne!(identities[1], identities[2]);
}

#[test]
fn exact_and_normalized_field_names_are_both_identity_inputs() {
    let abi = parse_abi(
        &[
            function("camel", &tuple("pair", "fooBar", "type", "tuple")),
            function("snake", &tuple("pair", "foo_bar", "type", "tuple")),
            function("empty", &tuple("pair", "", "type", "tuple")),
        ],
        HardLimits::V1,
    )
    .unwrap();
    let registry = OwnedTypeRegistry::build(&abi, &[], HardLimits::V1).unwrap();
    assert_eq!(registry.tuples().count(), 3);
    assert_eq!(rust_field("", 0), "value_0");
    assert_eq!(rust_field("type", 1), "r#type");
    assert_eq!(rust_field("fooBar", 0), rust_field("foo_bar", 0));
    let camel = abi
        .functions
        .iter()
        .find(|item| item.name == "camel")
        .unwrap();
    let snake = abi
        .functions
        .iter()
        .find(|item| item.name == "snake")
        .unwrap();
    assert_ne!(
        registry.identity(&camel.outputs[0].ty, None),
        registry.identity(&snake.outputs[0].ty, None)
    );
}

#[test]
fn verified_enrichment_and_abi_fallback_coexist() {
    let abi = parse_abi(
        &[
            function(
                "named",
                &tuple_with_internal("pair", "value", "struct Vault.Record"),
            ),
            function(
                "fallback",
                &tuple_with_internal("pair", "value", "struct Missing.Record"),
            ),
        ],
        HardLimits::V1,
    )
    .unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[{
        "nodeType":"StructDefinition", "name":"Record", "canonicalName":"Vault.Record",
        "members":[{"nodeType":"VariableDeclaration", "name":"value",
            "typeDescriptions":{"typeString":"uint256"}}]
    }]});
    let mut diagnostics = Diagnostics::new();
    let enrichments = enrich(&abi, Some(&ast), HardLimits::V1, &mut diagnostics).unwrap();
    let registry = OwnedTypeRegistry::build(&abi, &enrichments, HardLimits::V1).unwrap();
    assert_eq!(enrichments.len(), 1);
    assert_eq!(registry.tuples().count(), 1);
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == DiagnosticCode::AstEnrichmentUnavailable)
    );
}

#[test]
fn malformed_internal_suffix_cannot_reuse_another_verified_enrichment() {
    let abi = parse_abi(
        &[
            function(
                "verified",
                &tuple_with_internal("pair", "value", "struct Vault.Record"),
            ),
            function(
                "malformed",
                &tuple_with_internal("pair", "value", "struct Vault.Record[garbage]"),
            ),
        ],
        HardLimits::V1,
    )
    .unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[{
        "nodeType":"StructDefinition", "name":"Record", "canonicalName":"Vault.Record",
        "members":[{"nodeType":"VariableDeclaration", "name":"value",
            "typeDescriptions":{"typeString":"uint256"}}]
    }]});
    let mut diagnostics = Diagnostics::new();
    let enrichments = enrich(&abi, Some(&ast), HardLimits::V1, &mut diagnostics).unwrap();
    let registry = OwnedTypeRegistry::build(&abi, &enrichments, HardLimits::V1).unwrap();
    assert_eq!(enrichments.len(), 1);
    assert_eq!(registry.tuples().count(), 1);
    let malformed = &abi.functions[0].outputs[0];
    assert!(
        registry
            .resolved_enrichment(&malformed.ty, malformed.internal_type.as_deref())
            .is_none()
    );
}

#[test]
fn shortened_digest_collision_fails_closed() {
    fn collide(_: &[u8]) -> [u8; 16] {
        [7; 16]
    }

    let abi = decision_abi();
    let error =
        OwnedTypeRegistry::build_with_digest(&abi, &[], HardLimits::V1, collide).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

fn decision_abi() -> musubi_core::ContractAbi {
    parse_abi(
        &[
            function(
                "initial",
                &decision("shouldPlace", "struct LpvTypes.InitialPlacementDecision"),
            ),
            function(
                "rebalance",
                &decision("shouldRebalance", "struct LpvTypes.RebalanceDecision"),
            ),
        ],
        HardLimits::V1,
    )
    .unwrap()
}

fn decision(flag: &str, internal: &str) -> serde_json::Value {
    json!({"name":"decision","type":"tuple","internalType":internal,"components":[
        {"name":flag,"type":"bool"},
        {"name":"targetCenterTick","type":"int24"},
        {"name":"targetRangeWidth","type":"int24"}
    ]})
}

fn outer(name: &str, flag: &str, child_type: &str) -> serde_json::Value {
    json!({"name":"outer","type":"tuple","components":[{
        "name":name,"type":child_type,"components":[
            {"name":flag,"type":"bool"},{"name":"tick","type":"int24"}
        ]
    }]})
}

fn tuple(name: &str, first: &str, second: &str, ty: &str) -> serde_json::Value {
    json!({"name":name,"type":ty,"components":[
        {"name":first,"type":"uint256"},{"name":second,"type":"bool"}
    ]})
}

fn tuple_with_internal(name: &str, field: &str, internal: &str) -> serde_json::Value {
    json!({"name":name,"type":"tuple","internalType":internal,"components":[
        {"name":field,"type":"uint256"}
    ]})
}

fn function(name: &str, output: &serde_json::Value) -> serde_json::Value {
    json!({
        "type":"function", "name":name, "stateMutability":"view",
        "inputs":[], "outputs":[output]
    })
}
