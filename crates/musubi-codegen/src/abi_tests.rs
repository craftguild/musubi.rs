//! ABI wire-family, collision, and AST enrichment tests.

use musubi_core::{AbiType, ArtifactKind, StateMutability};
use serde_json::json;

use crate::{
    DiagnosticCode, Diagnostics, EnrichmentKind, HardLimits, abi_parse::parse_abi, ast::enrich,
    name_validation::validate_generated_names, owned_identity::OwnedTypeRegistry,
};

#[test]
fn abi_parses_recursive_wire_type_matrix() {
    let values = vec![json!({
        "type": "function",
        "name": "matrix",
        "stateMutability": "pure",
        "inputs": [
            {"name":"a","type":"address"}, {"name":"b","type":"bool"},
            {"name":"s","type":"string"}, {"name":"d","type":"bytes"},
            {"name":"f","type":"bytes32"}, {"name":"u","type":"uint8"},
            {"name":"i","type":"int256"}, {"name":"uf","type":"ufixed128x18"},
            {"name":"sf","type":"fixed32x5"}, {"name":"fp","type":"function"},
            {"name":"arr","type":"uint256[][2]"},
            {"name":"pair","type":"tuple","components":[
                {"name":"left","type":"address"},
                {"name":"right","type":"tuple[]","components":[{"name":"n","type":"uint16"}]}
            ]}
        ],
        "outputs": []
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let function = &abi.functions[0];
    assert_eq!(function.state_mutability, StateMutability::Pure);
    assert_eq!(function.inputs[5].ty, AbiType::Uint(8));
    assert!(matches!(function.inputs[10].ty, AbiType::FixedArray(_, 2)));
    assert!(matches!(function.inputs[11].ty, AbiType::Tuple(_)));
}

#[test]
fn abi_enforces_combined_tuple_and_array_nesting() {
    let arrays_64 = format!("uint256{}", "[]".repeat(64));
    assert!(parse_abi(&single_input(&arrays_64), HardLimits::V1).is_ok());

    let arrays_65 = format!("uint256{}", "[]".repeat(65));
    let error = parse_abi(&single_input(&arrays_65), HardLimits::V1).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::LimitExceeded);

    let mixed_64 = vec![json!({
        "type":"function", "name":"nested", "stateMutability":"pure", "outputs":[],
        "inputs":[{"name":"outer", "type":"tuple[]", "components":[{
            "name":"value", "type":format!("uint256{}", "[]".repeat(62))
        }]}]
    })];
    assert!(parse_abi(&mixed_64, HardLimits::V1).is_ok());
    let mixed_65 = vec![json!({
        "type":"function", "name":"nested", "stateMutability":"pure", "outputs":[],
        "inputs":[{"name":"outer", "type":"tuple[]", "components":[{
            "name":"value", "type":format!("uint256{}", "[]".repeat(63))
        }]}]
    })];
    assert_eq!(
        parse_abi(&mixed_65, HardLimits::V1).unwrap_err().code,
        DiagnosticCode::LimitExceeded
    );

    let extreme = format!("uint256{}", "[]".repeat(100_000));
    assert_eq!(
        parse_abi(&single_input(&extreme), HardLimits::V1)
            .unwrap_err()
            .code,
        DiagnosticCode::LimitExceeded
    );
}

fn single_input(ty: &str) -> Vec<serde_json::Value> {
    vec![json!({
        "type":"function", "name":"nested", "stateMutability":"pure",
        "inputs":[{"name":"value", "type":ty}], "outputs":[]
    })]
}

#[test]
fn abi_uses_known_selector_and_topic_vectors() {
    let values = vec![
        json!({"type":"function","name":"transfer","stateMutability":"nonpayable","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[{"name":"","type":"bool"}]}),
        json!({"type":"event","name":"Transfer","anonymous":false,"inputs":[{"name":"from","type":"address","indexed":true},{"name":"to","type":"address","indexed":true},{"name":"value","type":"uint256","indexed":false}]}),
    ];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    assert_eq!(abi.functions[0].selector, [0xa9, 0x05, 0x9c, 0xbb]);
    assert_eq!(
        hex::encode(abi.events[0].topic.unwrap()),
        "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
    );
}

#[test]
fn abi_rejects_context_invalid_standard_properties_and_topics() {
    let invalid = [
        json!({"type":"constructor","inputs":[],"stateMutability":"nonpayable","anonymous":false}),
        json!({"type":"constructor","inputs":[],"stateMutability":"view"}),
        json!({"type":"function","name":"read","inputs":[],"outputs":[],"stateMutability":"view","anonymous":false}),
        json!({"type":"function","name":"read","inputs":[{"name":"value","type":"uint256","indexed":false}],"outputs":[],"stateMutability":"view"}),
        json!({"type":"function","name":"read","inputs":[{"name":"value","type":"uint256","components":[{"name":"ignored","type":"uint256"}]}],"outputs":[],"stateMutability":"view"}),
        json!({"type":"function","name":"read","inputs":[{"name":"value","type":"tuple","components":[{"name":"nested","type":"uint256","indexed":false}]}],"outputs":[],"stateMutability":"view"}),
        json!({"type":"fallback","stateMutability":"pure"}),
        json!({"type":"event","name":"TooMany","anonymous":false,"inputs":[
            {"name":"a","type":"uint256","indexed":true},
            {"name":"b","type":"uint256","indexed":true},
            {"name":"c","type":"uint256","indexed":true},
            {"name":"d","type":"uint256","indexed":true}
        ]}),
        json!({"type":"event","name":"TooManyAnonymous","anonymous":true,"inputs":[
            {"name":"a","type":"uint256","indexed":true},
            {"name":"b","type":"uint256","indexed":true},
            {"name":"c","type":"uint256","indexed":true},
            {"name":"d","type":"uint256","indexed":true},
            {"name":"e","type":"uint256","indexed":true}
        ]}),
    ];
    for item in invalid {
        assert!(parse_abi(&[item], HardLimits::V1).is_err());
    }
}

#[test]
fn abi_rejects_real_selector_collision() {
    let values = vec![
        json!({"type":"function","name":"burn","inputs":[{"name":"","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"}),
        json!({"type":"function","name":"collate_propagate_storage","inputs":[{"name":"","type":"bytes16"}],"outputs":[],"stateMutability":"nonpayable"}),
    ];
    let error = parse_abi(&values, HardLimits::V1).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

#[test]
fn ast_enriches_enum_udvt_and_struct_without_changing_wire() {
    let values = vec![json!({
        "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
        "inputs":[
            {"name":"status","type":"uint8","internalType":"enum Vault.Status"},
            {"name":"amount","type":"uint256","internalType":"type Vault.Amount"},
            {"name":"record","type":"tuple","internalType":"struct Vault.Record","components":[{"name":"owner","type":"address"}]}
        ]
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"EnumDefinition","name":"Status","canonicalName":"Vault.Status","members":[{"nodeType":"EnumValue","name":"Open"},{"nodeType":"EnumValue","name":"Closed"}]},
        {"nodeType":"UserDefinedValueTypeDefinition","name":"Amount","canonicalName":"Vault.Amount","underlyingType":{"typeDescriptions":{"typeString":"uint256"}}},
        {"nodeType":"StructDefinition","name":"Record","canonicalName":"Vault.Record","members":[{"nodeType":"VariableDeclaration","name":"owner","typeDescriptions":{"typeString":"address"}}]}
    ]});
    let mut diagnostics = Diagnostics::new();
    let enriched = enrich(&abi, Some(&ast), HardLimits::V1, &mut diagnostics).unwrap();
    assert_eq!(enriched.len(), 3);
    assert!(
        enriched
            .iter()
            .any(|item| matches!(item.kind, EnrichmentKind::Enum { .. }))
    );
    assert!(!diagnostics.failed(false));
    assert_eq!(abi.functions[0].inputs[0].ty, AbiType::Uint(8));
}

#[test]
fn ast_wire_contradiction_fails_closed() {
    let values = vec![
        json!({"type":"function","name":"set","stateMutability":"nonpayable","outputs":[],"inputs":[{"name":"amount","type":"uint256","internalType":"type Vault.Amount"}]}),
    ];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[{"nodeType":"UserDefinedValueTypeDefinition","name":"Amount","canonicalName":"Vault.Amount","underlyingType":{"typeDescriptions":{"typeString":"address"}}}]});
    let error = enrich(&abi, Some(&ast), HardLimits::V1, &mut Diagnostics::new()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

#[test]
fn ast_nested_struct_array_contradiction_fails_closed() {
    let values = vec![json!({
        "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
        "inputs":[{"name":"outer", "type":"tuple", "internalType":"struct Vault.Outer",
          "components":[{"name":"children", "type":"tuple[]", "internalType":"struct Vault.Inner[]",
            "components":[{"name":"value", "type":"uint256"}]}]}]
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"StructDefinition","name":"Outer","canonicalName":"Vault.Outer","members":[
            {"nodeType":"VariableDeclaration","name":"children","typeDescriptions":{"typeString":"struct Vault.Inner[] memory"}}
        ]},
        {"nodeType":"StructDefinition","name":"Inner","canonicalName":"Vault.Inner","members":[
            {"nodeType":"VariableDeclaration","name":"value","typeDescriptions":{"typeString":"address"}}
        ]}
    ]});
    let error = enrich(&abi, Some(&ast), HardLimits::V1, &mut Diagnostics::new()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

#[test]
fn ast_unknown_integer_disables_entire_enrichment_with_warning() {
    let values = vec![json!({
        "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
        "inputs":[{"name":"record", "type":"tuple", "internalType":"struct Vault.Record",
          "components":[{"name":"value", "type":"uint256"}]}]
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"StructDefinition","name":"Record","canonicalName":"Vault.Record","members":[
            {"nodeType":"VariableDeclaration","name":"value","typeDescriptions":{"typeString":"uint25x"}}
        ]}
    ]});
    let mut diagnostics = Diagnostics::new();
    let enrichments = enrich(&abi, Some(&ast), HardLimits::V1, &mut diagnostics).unwrap();
    assert!(enrichments.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == DiagnosticCode::AstEnrichmentUnavailable })
    );
}

#[test]
fn ast_exact_namespace_and_recursive_array_resolution_prevent_false_match() {
    let values = vec![
        json!({
            "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
            "inputs":[{"name":"record", "type":"tuple", "internalType":"struct A.Record",
              "components":[{"name":"values", "type":"uint256[][2]"}]}]
        }),
        json!({"type":"event","name":"Record","anonymous":false,"inputs":[]}),
    ];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"StructDefinition","name":"Record","canonicalName":"A.Record","members":[
            {"nodeType":"VariableDeclaration","name":"values","typeDescriptions":{"typeString":"uint256[][2] memory"}}
        ]},
        {"nodeType":"StructDefinition","name":"Record","canonicalName":"B.Record","members":[
            {"nodeType":"VariableDeclaration","name":"values","typeDescriptions":{"typeString":"address"}}
        ]}
    ]});
    let enrichments = enrich(&abi, Some(&ast), HardLimits::V1, &mut Diagnostics::new()).unwrap();
    assert_eq!(enrichments.len(), 1);
    assert_eq!(enrichments[0].internal_type, "struct A.Record");
    let owned = OwnedTypeRegistry::build(&abi, &enrichments, HardLimits::V1).unwrap();
    assert!(
        validate_generated_names(
            "vault",
            ArtifactKind::Deployable,
            &abi,
            &enrichments,
            &owned
        )
        .is_err()
    );
}

#[test]
fn ast_rejects_source_injection_in_canonical_identity() {
    let identity = "Vault.Record\npub struct Injected; //";
    let values = vec![json!({
        "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
        "inputs":[{"name":"record", "type":"tuple", "internalType":format!("struct {identity}"),
          "components":[{"name":"value", "type":"uint256"}]}]
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"StructDefinition","name":"Record","canonicalName":identity,"members":[
            {"nodeType":"VariableDeclaration","name":"value","typeDescriptions":{"typeString":"uint256"}}
        ]}
    ]});
    let error = enrich(&abi, Some(&ast), HardLimits::V1, &mut Diagnostics::new()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

#[test]
fn ast_unknown_member_cannot_hide_a_later_known_contradiction() {
    let values = vec![json!({
        "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
        "inputs":[{"name":"record", "type":"tuple", "internalType":"struct Vault.Record",
          "components":[{"name":"first", "type":"uint256"},{"name":"second", "type":"uint256"}]}]
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"StructDefinition","name":"Record","canonicalName":"Vault.Record","members":[
            {"nodeType":"VariableDeclaration","name":"first","typeDescriptions":{"typeString":"mystery"}},
            {"nodeType":"VariableDeclaration","name":"second","typeDescriptions":{"typeString":"address"}}
        ]}
    ]});
    let error = enrich(&abi, Some(&ast), HardLimits::V1, &mut Diagnostics::new()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InconsistentArtifact);
}

#[test]
fn ast_noncanonical_type_spellings_disable_enrichment() {
    for malformed in [
        "uint0256",
        "uint+256",
        "bytes+1",
        "uint256[02]",
        "uint256[+2]",
        "uint256 [2]",
        "contract ???",
        "function garbage",
        "function () external garbage",
        "function (??? garbage) external returns (!!!)",
        "function (uint256) external pure view",
    ] {
        let wire = match malformed {
            "uint256[02]" | "uint256[+2]" | "uint256 [2]" => "uint256[2]",
            "bytes+1" => "bytes1",
            "contract ???" => "address",
            "function garbage"
            | "function () external garbage"
            | "function (??? garbage) external returns (!!!)"
            | "function (uint256) external pure view" => "function",
            _ => "uint256",
        };
        let values = vec![json!({
            "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
            "inputs":[{"name":"record", "type":"tuple", "internalType":"struct Vault.Record",
              "components":[{"name":"value", "type":wire}]}]
        })];
        let abi = parse_abi(&values, HardLimits::V1).unwrap();
        let ast = json!({"nodeType":"SourceUnit","nodes":[
            {"nodeType":"StructDefinition","name":"Record","canonicalName":"Vault.Record","members":[
                {"nodeType":"VariableDeclaration","name":"value","typeDescriptions":{"typeString":malformed}}
            ]}
        ]});
        let mut diagnostics = Diagnostics::new();
        assert!(
            enrich(&abi, Some(&ast), HardLimits::V1, &mut diagnostics)
                .unwrap()
                .is_empty()
        );
        assert!(
            diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::AstEnrichmentUnavailable)
        );
    }
}

#[test]
fn ast_malformed_internal_type_array_suffix_disables_enrichment() {
    let values = vec![json!({
        "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
        "inputs":[{"name":"record", "type":"tuple", "internalType":"struct Vault.Record[garbage]",
          "components":[{"name":"value", "type":"uint256"}]}]
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"StructDefinition","name":"Record","canonicalName":"Vault.Record","members":[
            {"nodeType":"VariableDeclaration","name":"value","typeDescriptions":{"typeString":"uint256"}}
        ]}
    ]});
    let mut diagnostics = Diagnostics::new();
    assert!(
        enrich(&abi, Some(&ast), HardLimits::V1, &mut diagnostics)
            .unwrap()
            .is_empty()
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == DiagnosticCode::AstEnrichmentUnavailable)
    );
}

#[test]
fn ast_accepts_solidity_dollar_identifiers_and_normalizes_rust_names() {
    let values = vec![json!({
        "type":"function", "name":"set", "stateMutability":"nonpayable", "outputs":[],
        "inputs":[{"name":"record", "type":"tuple", "internalType":"struct $Vault.$Record",
          "components":[{"name":"$value", "type":"uint256"},{"name":"$callback", "type":"function"}]}]
    })];
    let abi = parse_abi(&values, HardLimits::V1).unwrap();
    let ast = json!({"nodeType":"SourceUnit","nodes":[
        {"nodeType":"StructDefinition","name":"$Record","canonicalName":"$Vault.$Record","members":[
            {"nodeType":"VariableDeclaration","name":"$value","typeDescriptions":{"typeString":"uint256"}},
            {"nodeType":"VariableDeclaration","name":"$callback","typeDescriptions":{"typeString":"function (uint256,address) external view returns (bool)"}}
        ]}
    ]});
    let enrichments = enrich(&abi, Some(&ast), HardLimits::V1, &mut Diagnostics::new()).unwrap();
    assert_eq!(enrichments[0].rust_name, "DollarRecord");
    let owned = OwnedTypeRegistry::build(&abi, &enrichments, HardLimits::V1).unwrap();
    assert!(
        validate_generated_names(
            "vault",
            ArtifactKind::Deployable,
            &abi,
            &enrichments,
            &owned
        )
        .is_ok()
    );
}
