//! Preflight validation for every generated Rust namespace and struct field scope.

use std::collections::BTreeSet;

use musubi_core::{AbiField, AbiType, ArtifactKind, ContractAbi, StateMutability};

use crate::{
    Diagnostic, DiagnosticCode, Enrichment, EnrichmentKind,
    names::{rust_field, rust_pascal, rust_variant},
    owned_identity::OwnedTypeRegistry,
    render_names::{error_type_name, event_type_name, function_method_name, function_type_name},
    render_type_imports::type_import_names,
};

pub(crate) fn validate_generated_names(
    contract_id: &str,
    kind: ArtifactKind,
    abi: &ContractAbi,
    enrichments: &[Enrichment],
    owned_types: &OwnedTypeRegistry,
) -> Result<(), Diagnostic> {
    validate_methods(abi, kind)?;
    validate_types(contract_id, kind, abi, enrichments, owned_types)?;
    validate_error_variants(abi)?;
    validate_enrichment_variants(enrichments)?;
    validate_all_fields(abi, enrichments)
}

fn validate_methods(abi: &ContractAbi, kind: ArtifactKind) -> Result<(), Diagnostic> {
    let mut names = BTreeSet::from([
        "address".to_owned(),
        "at".into(),
        "decode_error".into(),
        "errors".into(),
        "events".into(),
    ]);
    if kind == ArtifactKind::Deployable {
        names.extend(["deployment".into(), "init_code".into()]);
    }
    for function in &abi.functions {
        insert(&mut names, function_method_name(abi, function))?;
    }
    Ok(())
}

fn validate_types(
    contract_id: &str,
    kind: ArtifactKind,
    abi: &ContractAbi,
    enrichments: &[Enrichment],
    owned_types: &OwnedTypeRegistry,
) -> Result<(), Diagnostic> {
    let marker = rust_pascal(contract_id);
    let mut names = BTreeSet::from(
        [
            "Address",
            "Bytes",
            "ExternalArtifactCoherence",
            "Result",
            "String",
            "Vec",
        ]
        .map(str::to_owned),
    );
    if kind == ArtifactKind::Deployable {
        names.extend(["ContractArtifact", "TransactionRequest", "U256"].map(str::to_owned));
    }
    names.extend(type_import_names(abi, enrichments, owned_types));
    if abi.functions.iter().any(|function| {
        matches!(
            function.state_mutability,
            StateMutability::View | StateMutability::Pure
        )
    }) {
        names.insert("FunctionCall".into());
    }
    if abi.functions.iter().any(|function| {
        !matches!(
            function.state_mutability,
            StateMutability::View | StateMutability::Pure
        )
    }) {
        names.insert("FunctionTransaction".into());
    }
    for name in [
        marker.clone(),
        format!("{marker}At"),
        format!("{marker}Error"),
    ] {
        insert(&mut names, name)?;
    }
    if !abi.functions.is_empty() {
        insert(&mut names, format!("{marker}Client"))?;
    }
    if abi.constructor.is_some() {
        insert(&mut names, "ConstructorArgs".into())?;
    }
    for function in &abi.functions {
        let base = function_type_name(abi, function);
        if !function.inputs.is_empty() {
            insert(&mut names, format!("{base}Args"))?;
        }
        if !function.outputs.is_empty() {
            insert(&mut names, format!("{base}Return"))?;
        }
    }
    for event in &abi.events {
        insert(&mut names, event_type_name(abi, event))?;
    }
    for error in &abi.errors {
        insert(&mut names, error_type_name(abi, error))?;
    }
    for enrichment in enrichments {
        insert(&mut names, enrichment.rust_name.clone())?;
    }
    for tuple in owned_types.tuples() {
        insert(&mut names, tuple.rust_name.clone())?;
    }
    Ok(())
}

fn validate_error_variants(abi: &ContractAbi) -> Result<(), Diagnostic> {
    let mut variants = BTreeSet::from(["Error".to_owned(), "Panic".into(), "Unknown".into()]);
    for error in &abi.errors {
        insert(&mut variants, error_type_name(abi, error))?;
    }
    Ok(())
}

fn validate_enrichment_variants(enrichments: &[Enrichment]) -> Result<(), Diagnostic> {
    for enrichment in enrichments {
        if let EnrichmentKind::Enum { variants, .. } = &enrichment.kind {
            let mut generated = BTreeSet::new();
            for variant in variants {
                insert(&mut generated, rust_variant(variant))?;
            }
        }
    }
    Ok(())
}

fn validate_all_fields(abi: &ContractAbi, enrichments: &[Enrichment]) -> Result<(), Diagnostic> {
    if let Some(constructor) = &abi.constructor {
        validate_fields(&constructor.inputs)?;
    }
    for function in &abi.functions {
        validate_fields(&function.inputs)?;
        validate_fields(&function.outputs)?;
    }
    for event in &abi.events {
        validate_field_iter(event.inputs.iter().map(|field| &field.field))?;
        for field in &event.inputs {
            validate_nested_fields(&field.field.ty)?;
        }
    }
    for error in &abi.errors {
        validate_fields(&error.inputs)?;
    }
    for enrichment in enrichments {
        if let EnrichmentKind::Struct { fields } = &enrichment.kind {
            validate_fields(fields)?;
        }
    }
    Ok(())
}

fn validate_fields(fields: &[AbiField]) -> Result<(), Diagnostic> {
    validate_field_iter(fields.iter())?;
    for field in fields {
        validate_nested_fields(&field.ty)?;
    }
    Ok(())
}

fn validate_nested_fields(ty: &AbiType) -> Result<(), Diagnostic> {
    match ty {
        AbiType::Array(element) | AbiType::FixedArray(element, _) => {
            validate_nested_fields(element)
        }
        AbiType::Tuple(fields) => validate_fields(fields),
        _ => Ok(()),
    }
}

fn validate_field_iter<'a>(fields: impl Iterator<Item = &'a AbiField>) -> Result<(), Diagnostic> {
    let mut generated = BTreeSet::new();
    for (index, field) in fields.enumerate() {
        insert(&mut generated, rust_field(&field.name, index))?;
    }
    Ok(())
}

fn insert(names: &mut BTreeSet<String>, name: String) -> Result<(), Diagnostic> {
    if names.insert(name) {
        Ok(())
    } else {
        Err(collision())
    }
}

fn collision() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InconsistentArtifact,
        "ABI or AST names collide in a generated Rust namespace",
    )
}
