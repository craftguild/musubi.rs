//! Rust-source compatibility projection of ABI fields and verified AST enrichments.

use std::collections::BTreeSet;

use musubi_core::{AbiField, AbiType, ContractAbi};

use crate::{Enrichment, HardLimits, ast_internal::matching_internal_base};

pub(super) fn rust_enrichment_view(enrichments: &[Enrichment]) -> Vec<Enrichment> {
    let mut view = enrichments.to_vec();
    let names = enrichments
        .iter()
        .map(|item| item.internal_type.as_str())
        .collect::<BTreeSet<_>>();
    for enrichment in &mut view {
        if let crate::EnrichmentKind::Struct { fields } = &mut enrichment.kind {
            normalize_fields(fields, &names);
        }
    }
    view
}

pub(super) fn rust_abi_view(abi: &ContractAbi, enrichments: &[Enrichment]) -> ContractAbi {
    let mut view = abi.clone();
    let names = enrichments
        .iter()
        .map(|item| item.internal_type.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(constructor) = &mut view.constructor {
        normalize_fields(&mut constructor.inputs, &names);
    }
    for function in &mut view.functions {
        normalize_fields(&mut function.inputs, &names);
        normalize_fields(&mut function.outputs, &names);
    }
    for event in &mut view.events {
        for field in &mut event.inputs {
            normalize_field(&mut field.field, &names);
        }
    }
    for error in &mut view.errors {
        normalize_fields(&mut error.inputs, &names);
    }
    view
}

fn normalize_fields(fields: &mut [AbiField], enrichments: &BTreeSet<&str>) {
    for field in fields {
        normalize_field(field, enrichments);
    }
}

fn normalize_field(field: &mut AbiField, enrichments: &BTreeSet<&str>) {
    let resolved = field.internal_type.as_deref().and_then(|internal| {
        matching_internal_base(&field.ty, internal, HardLimits::V1.nesting)
            .filter(|base| enrichments.contains(base))
            .map(str::to_owned)
    });
    field.internal_type = resolved;
    if field.internal_type.is_some() {
        erase_nested_surface(&mut field.ty);
    } else {
        normalize_nested_surface(&mut field.ty, enrichments);
    }
}

fn normalize_nested_surface(ty: &mut AbiType, enrichments: &BTreeSet<&str>) {
    match ty {
        AbiType::Array(element) | AbiType::FixedArray(element, _) => {
            normalize_nested_surface(element, enrichments);
        }
        AbiType::Tuple(fields) => normalize_fields(fields, enrichments),
        _ => {}
    }
}

fn erase_nested_surface(ty: &mut AbiType) {
    match ty {
        AbiType::Array(element) | AbiType::FixedArray(element, _) => erase_nested_surface(element),
        AbiType::Tuple(fields) => {
            for field in fields {
                field.name.clear();
                field.internal_type = None;
                erase_nested_surface(&mut field.ty);
            }
        }
        _ => {}
    }
}
