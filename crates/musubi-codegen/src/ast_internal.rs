//! Bounded parsing and ABI comparison for AST `internalType` identities.

use std::collections::BTreeMap;

use musubi_core::{AbiField, AbiType, ContractAbi};

use crate::ast_type::valid_qualified;

pub(super) fn internal_type_occurrences(
    abi: &ContractAbi,
    maximum_nesting: usize,
) -> BTreeMap<String, Vec<&AbiField>> {
    let mut output = BTreeMap::new();
    for field in all_fields(abi) {
        collect_field(field, maximum_nesting, &mut output);
    }
    output
}

fn collect_field<'a>(
    field: &'a AbiField,
    maximum_nesting: usize,
    output: &mut BTreeMap<String, Vec<&'a AbiField>>,
) {
    if let Some(internal) = &field.internal_type {
        let identity =
            parse_internal(internal, maximum_nesting).map_or(internal.as_str(), |(base, _)| base);
        output.entry(identity.to_owned()).or_default().push(field);
    }
    match &field.ty {
        AbiType::Tuple(fields) => {
            for child in fields {
                collect_field(child, maximum_nesting, output);
            }
        }
        AbiType::Array(element) | AbiType::FixedArray(element, _) => {
            collect_type(element, maximum_nesting, output);
        }
        _ => {}
    }
}

fn collect_type<'a>(
    ty: &'a AbiType,
    maximum_nesting: usize,
    output: &mut BTreeMap<String, Vec<&'a AbiField>>,
) {
    match ty {
        AbiType::Tuple(fields) => {
            for field in fields {
                collect_field(field, maximum_nesting, output);
            }
        }
        AbiType::Array(element) | AbiType::FixedArray(element, _) => {
            collect_type(element, maximum_nesting, output);
        }
        _ => {}
    }
}

pub(super) fn internal_wrappers_match(field: &AbiField, maximum_nesting: usize) -> bool {
    field.internal_type.as_deref().is_some_and(|internal| {
        matching_internal_base(&field.ty, internal, maximum_nesting).is_some()
    })
}

pub(crate) fn matching_internal_base<'a>(
    ty: &AbiType,
    internal: &'a str,
    maximum_nesting: usize,
) -> Option<&'a str> {
    let (base, dimensions) = parse_internal(internal, maximum_nesting)?;
    let mut wire = ty;
    for dimension in dimensions.iter().rev() {
        wire = match (dimension, wire) {
            (None, AbiType::Array(element)) => element,
            (Some(left), AbiType::FixedArray(element, right)) if left == right => element,
            _ => return None,
        };
    }
    (!matches!(wire, AbiType::Array(_) | AbiType::FixedArray(_, _))).then_some(base)
}

pub(super) fn wire_base(mut ty: &AbiType) -> &AbiType {
    while let AbiType::Array(element) | AbiType::FixedArray(element, _) = ty {
        ty = element;
    }
    ty
}

pub(super) fn split_internal(
    value: &str,
    identifier_limit: usize,
    maximum_nesting: usize,
) -> Option<(&str, &str)> {
    let (base, _) = parse_internal(value, maximum_nesting)?;
    ["struct", "enum", "type"].into_iter().find_map(|prefix| {
        base.strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix(' '))
            .filter(|identity| valid_qualified(identity, identifier_limit))
            .map(|identity| (prefix, identity))
    })
}

fn parse_internal(value: &str, maximum_nesting: usize) -> Option<(&str, Vec<Option<usize>>)> {
    let base_end = value.find('[').unwrap_or(value.len());
    let base = &value[..base_end];
    if base.is_empty() || base.ends_with(char::is_whitespace) || base.contains(']') {
        return None;
    }
    let mut suffix = &value[base_end..];
    let mut dimensions = Vec::new();
    while !suffix.is_empty() {
        let suffix_body = suffix.strip_prefix('[')?;
        let end = suffix_body.find(']')?;
        let length = &suffix_body[..end];
        let dimension = if length.is_empty() {
            None
        } else {
            if !length.bytes().all(|byte| byte.is_ascii_digit()) || length.starts_with('0') {
                return None;
            }
            Some(length.parse::<usize>().ok()?)
        };
        dimensions.push(dimension);
        if dimensions.len() > maximum_nesting {
            return None;
        }
        suffix = &suffix_body[end + 1..];
    }
    Some((base, dimensions))
}

fn all_fields(abi: &ContractAbi) -> impl Iterator<Item = &AbiField> {
    abi.constructor
        .iter()
        .flat_map(|item| item.inputs.iter())
        .chain(
            abi.functions
                .iter()
                .flat_map(|item| item.inputs.iter().chain(item.outputs.iter())),
        )
        .chain(
            abi.events
                .iter()
                .flat_map(|item| item.inputs.iter().map(|field| &field.field)),
        )
        .chain(abi.errors.iter().flat_map(|item| item.inputs.iter()))
}
