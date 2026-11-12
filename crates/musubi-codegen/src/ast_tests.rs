//! AST function and `internalType` boundary regression tests.

use serde_json::json;

use crate::ast_type_parse::{parse_calls, reset_parse_calls};
use crate::{DiagnosticCode, Diagnostics, HardLimits, abi_parse::parse_abi, ast::enrich};

#[test]
fn internal_function_pointer_contradicts_abi_function_pointer() {
    let abi = record_abi("tuple", "struct Vault.Record");
    let ast = record_ast("function () internal");

    let error = enrich(&abi, Some(&ast), HardLimits::V1, &mut Diagnostics::new()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

#[test]
fn internal_type_array_wrappers_must_match_abi_wrappers() {
    for (wire, internal) in [
        ("tuple", "struct Vault.Record[2]"),
        ("tuple[3]", "struct Vault.Record[2]"),
        ("tuple[]", "struct Vault.Record"),
    ] {
        let abi = record_abi(wire, internal);
        let error = enrich(
            &abi,
            Some(&record_ast("uint256")),
            HardLimits::V1,
            &mut Diagnostics::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
    }

    let abi = record_abi("tuple[2][]", "struct Vault.Record[2][]");
    let enrichments = enrich(
        &abi,
        Some(&record_ast("uint256")),
        HardLimits::V1,
        &mut Diagnostics::new(),
    )
    .unwrap();
    assert_eq!(enrichments.len(), 1);
}

#[test]
fn internal_type_suffix_nesting_is_bounded_without_panicking() {
    for dimensions in [65, 100_000] {
        let internal = format!("struct Vault.Record{}", "[]".repeat(dimensions));
        let abi = record_abi("tuple", &internal);
        let mut diagnostics = Diagnostics::new();
        let enrichments = enrich(
            &abi,
            Some(&record_ast("uint256")),
            HardLimits::V1,
            &mut diagnostics,
        )
        .unwrap();
        assert!(enrichments.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == DiagnosticCode::AstEnrichmentUnavailable })
        );
    }
}

#[test]
fn nested_function_signature_obeys_ast_nesting_limit() {
    let abi = function_record_abi();
    let accepted = nested_function(64);
    let enrichments = enrich(
        &abi,
        Some(&record_ast(&accepted)),
        HardLimits::V1,
        &mut Diagnostics::new(),
    )
    .unwrap();
    assert_eq!(enrichments.len(), 1);

    for depth in [65, 20_000] {
        let mut diagnostics = Diagnostics::new();
        let enrichments = enrich(
            &abi,
            Some(&record_ast(&nested_function(depth))),
            HardLimits::V1,
            &mut diagnostics,
        )
        .unwrap();
        assert!(enrichments.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == DiagnosticCode::AstEnrichmentUnavailable })
        );
    }
}

#[test]
fn long_unknown_ast_type_is_parsed_once_for_many_occurrences() {
    let inputs = (0..4_096)
        .map(|index| {
            json!({
                "name":format!("record{index}"), "type":"tuple",
                "internalType":"struct Vault.Record",
                "components":[{"name":"value", "type":"uint256"}]
            })
        })
        .collect::<Vec<_>>();
    let abi = parse_abi(
        &[json!({
            "type":"function", "name":"set", "stateMutability":"nonpayable",
            "inputs":inputs, "outputs":[]
        })],
        HardLimits::V1,
    )
    .unwrap();
    let long_unknown = format!("function ({}) external", "x".repeat(1024 * 1024));
    let mut diagnostics = Diagnostics::new();

    reset_parse_calls();
    let enrichments = enrich(
        &abi,
        Some(&record_ast(&long_unknown)),
        HardLimits::V1,
        &mut diagnostics,
    )
    .unwrap();

    assert!(enrichments.is_empty());
    assert_eq!(parse_calls(), 1);
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == DiagnosticCode::AstEnrichmentUnavailable)
    );
}

#[test]
fn repeated_identity_requires_matching_abi_structures_before_ast_fallback() {
    let abi = parse_abi(
        &[json!({
            "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
            "inputs":[
                {"name":"left", "type":"tuple", "internalType":"struct Vault.Record",
                    "components":[{"name":"value", "type":"uint256"}]},
                {"name":"right", "type":"tuple", "internalType":"struct Vault.Record",
                    "components":[{"name":"value", "type":"address"}]}
            ]
        })],
        HardLimits::V1,
    )
    .unwrap();

    let error = enrich(
        &abi,
        Some(&record_ast("unknown")),
        HardLimits::V1,
        &mut Diagnostics::new(),
    )
    .unwrap_err();

    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

fn record_abi(wire: &str, internal: &str) -> musubi_core::ContractAbi {
    parse_abi(
        &[json!({
            "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
            "inputs":[{"name":"record", "type":wire, "internalType":internal,
                "components":[{"name":"value", "type":"uint256"}]}]
        })],
        HardLimits::V1,
    )
    .unwrap()
}

fn function_record_abi() -> musubi_core::ContractAbi {
    parse_abi(
        &[json!({
            "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
            "inputs":[{"name":"record", "type":"tuple", "internalType":"struct Vault.Record",
                "components":[{"name":"value", "type":"function"}]}]
        })],
        HardLimits::V1,
    )
    .unwrap()
}

fn record_ast(member_type: &str) -> serde_json::Value {
    json!({"nodeType":"SourceUnit","nodes":[{
        "nodeType":"StructDefinition", "name":"Record", "canonicalName":"Vault.Record",
        "members":[{"nodeType":"VariableDeclaration", "name":"value",
            "typeDescriptions":{"typeString":member_type}}]
    }]})
}

fn nested_function(depth: usize) -> String {
    format!(
        "{}uint256{}",
        "function (".repeat(depth),
        ") external".repeat(depth)
    )
}
