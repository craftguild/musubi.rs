//! Closed typed-data schema parsing, graph checks, and canonical IR construction.
use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{TypedDataField, TypedDataRoot, TypedDataStruct, TypedDataType, sha256};
use serde::Deserialize;

use crate::{
    Diagnostic, DiagnosticCode, HardLimits, IrError, TypedDataInput, TypedDataIr,
    input::one,
    names::{module_name, reserved_typed_data_type_names, rust_field, rust_pascal},
};

const SCHEMA_VERSION: &str = "musubi/typed-data-schema-v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaWire {
    schema_version: String,
    types: Vec<TypeWire>,
    roots: Vec<TypedDataRoot>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TypeWire {
    name: String,
    fields: Vec<FieldWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldWire {
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

pub(crate) fn build(
    input: TypedDataInput,
    limits: HardLimits,
    maximum_evidence_bytes: usize,
) -> Result<(TypedDataIr, usize), IrError> {
    let document = crate::closed_json::parse(&input.schema)
        .map_err(|()| failure("typed-data schema is not valid closed-schema JSON"))?;
    let schema: SchemaWire = serde_json::from_value(document)
        .map_err(|_| failure("typed-data schema is not valid closed-schema JSON"))?;
    if schema.schema_version != SCHEMA_VERSION
        || schema.types.is_empty()
        || schema.roots.is_empty()
        || schema.types.len() > limits.typed_data_types
        || schema.roots.len() > limits.typed_data_types
    {
        return Err(failure(
            "typed-data schema version or entry count is invalid",
        ));
    }
    let names = validate_type_names(&schema.types, limits)?;
    let types = parse_types(schema.types, &names, limits)?;
    let graph = musubi_runtime::TypedDataGraph::new(&types)
        .map_err(|_| failure("typed-data graph is invalid or too deeply nested"))?;
    if graph.maximum_nesting() > limits.nesting {
        return Err(failure(
            "typed-data graph exceeds the effective nesting limit",
        ));
    }
    if graph.maximum_fixed_array_elements() > limits.typed_data_fixed_array_elements {
        return Err(failure(
            "typed-data graph exceeds the effective fixed-array limit",
        ));
    }
    let mut roots = schema.roots;
    roots.sort_by(|left, right| {
        (left.primary_type(), left.domain_type()).cmp(&(right.primary_type(), right.domain_type()))
    });
    validate_roots_and_reachability(&types, &roots)?;
    let (root_evidence, evidence_bytes) =
        crate::typed_data_evidence::build(&types, &roots, maximum_evidence_bytes)?;
    let evidence = crate::typed_data_vectors::verify(
        &input.vectors,
        &types,
        &roots,
        &root_evidence,
        limits.typed_data_vectors,
    )?;
    let module =
        module_name(Some(&input.config.rust_module), &input.config.id).map_err(diagnostic)?;
    Ok((
        TypedDataIr {
            id: input.config.id,
            module,
            types,
            roots,
            root_evidence,
            schema_sha256: sha256(&input.schema),
            vectors_sha256: sha256(&input.vectors),
            evidence,
        },
        evidence_bytes,
    ))
}

fn validate_type_names(
    types: &[TypeWire],
    limits: HardLimits,
) -> Result<BTreeSet<String>, IrError> {
    let mut names = BTreeSet::new();
    let mut rust_names = reserved_typed_data_type_names();
    for ty in types {
        identifier(&ty.name, limits)?;
        if !names.insert(ty.name.clone()) || !rust_names.insert(rust_pascal(&ty.name)) {
            return Err(failure("typed-data type names collide"));
        }
    }
    Ok(names)
}

fn parse_types(
    wire: Vec<TypeWire>,
    names: &BTreeSet<String>,
    limits: HardLimits,
) -> Result<Vec<TypedDataStruct>, IrError> {
    let mut components = 0_usize;
    let mut output = Vec::with_capacity(wire.len());
    for ty in wire {
        if ty.fields.is_empty() {
            return Err(failure(
                "typed-data structs must contain at least one field",
            ));
        }
        components = components
            .checked_add(ty.fields.len())
            .ok_or_else(|| failure("typed-data component count overflowed"))?;
        if components > limits.typed_data_fields {
            return Err(failure(
                "typed-data component count exceeds the effective limit",
            ));
        }
        let mut fields = BTreeSet::new();
        let mut rust_fields = BTreeSet::new();
        let parsed = ty
            .fields
            .into_iter()
            .enumerate()
            .map(|(index, field)| {
                identifier(&field.name, limits)?;
                if !fields.insert(field.name.clone())
                    || !rust_fields.insert(rust_field(&field.name, index))
                {
                    return Err(failure("typed-data field names collide"));
                }
                Ok(TypedDataField {
                    name: field.name,
                    ty: parse_field_type(
                        &field.ty,
                        names,
                        limits.nesting,
                        limits.typed_data_fixed_array_elements,
                        0,
                        1,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, IrError>>()?;
        output.push(TypedDataStruct {
            name: ty.name,
            fields: parsed,
        });
    }
    output.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(output)
}

fn parse_field_type(
    value: &str,
    names: &BTreeSet<String>,
    maximum_depth: usize,
    maximum_fixed_elements: usize,
    depth: usize,
    fixed_elements: usize,
) -> Result<TypedDataType, IrError> {
    if depth > maximum_depth
        || value.is_empty()
        || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(failure(
            "typed-data field type is malformed or too deeply nested",
        ));
    }
    if let Some(prefix) = value.strip_suffix(']') {
        let start = prefix
            .rfind('[')
            .ok_or_else(|| failure("typed-data array type is malformed"))?;
        let length = &prefix[start + 1..];
        let length = if length.is_empty() {
            None
        } else {
            if !canonical_decimal(length) {
                return Err(failure("typed-data fixed array length is invalid"));
            }
            Some(
                length
                    .parse::<usize>()
                    .map_err(|_| failure("typed-data fixed array length is invalid"))?,
            )
        };
        let child_fixed_elements = length.map_or(Ok(fixed_elements), |length| {
            fixed_elements
                .checked_mul(length)
                .filter(|product| length != 0 && *product <= maximum_fixed_elements)
                .ok_or_else(|| {
                    failure("typed-data fixed array exceeds the effective element limit")
                })
        })?;
        let element = parse_field_type(
            &prefix[..start],
            names,
            maximum_depth,
            maximum_fixed_elements,
            depth + 1,
            child_fixed_elements,
        )?;
        return if let Some(length) = length {
            Ok(TypedDataType::FixedArray(Box::new(element), length))
        } else {
            Ok(TypedDataType::Array(Box::new(element)))
        };
    }
    match value {
        "address" => Ok(TypedDataType::Address),
        "bool" => Ok(TypedDataType::Bool),
        "string" => Ok(TypedDataType::String),
        "bytes" => Ok(TypedDataType::Bytes),
        _ if value.starts_with("bytes") => {
            parse_width(&value[5..], 1, 32).map(TypedDataType::FixedBytes)
        }
        _ if value.starts_with("uint") => parse_width(&value[4..], 8, 256).map(TypedDataType::Uint),
        _ if value.starts_with("int") => parse_width(&value[3..], 8, 256).map(TypedDataType::Int),
        _ if names.contains(value) => Ok(TypedDataType::Struct(value.into())),
        _ => Err(failure(
            "typed-data field type is unsupported or unresolved",
        )),
    }
}

fn parse_width(value: &str, minimum: usize, maximum: usize) -> Result<usize, IrError> {
    if !canonical_decimal(value) {
        return Err(failure("typed-data integer or bytes width is invalid"));
    }
    let width = value
        .parse::<usize>()
        .map_err(|_| failure("typed-data integer or bytes width is invalid"))?;
    if !(minimum..=maximum).contains(&width) || (maximum == 256 && width % 8 != 0) {
        Err(failure("typed-data integer or bytes width is invalid"))
    } else {
        Ok(width)
    }
}

fn canonical_decimal(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value.len() == 1 || !value.starts_with('0'))
}

pub(crate) fn validate_roots_and_reachability(
    types: &[TypedDataStruct],
    roots: &[TypedDataRoot],
) -> Result<(), IrError> {
    let by_name = types
        .iter()
        .map(|ty| (ty.name.as_str(), ty))
        .collect::<BTreeMap<_, _>>();
    let mut unique = BTreeSet::new();
    let mut reachable = BTreeSet::new();
    let mut rust_methods = BTreeMap::<&str, BTreeSet<String>>::new();
    for root in roots {
        let key = (root.primary_type(), root.domain_type());
        if !unique.insert(key) || !by_name.contains_key(root.primary_type()) {
            return Err(failure("typed-data root is duplicate or unresolved"));
        }
        if let Some(method) = crate::names::typed_data_digest_method(root, roots)
            && !rust_methods
                .entry(root.primary_type())
                .or_default()
                .insert(method)
        {
            return Err(failure("typed-data root method names collide"));
        }
        visit(
            root.primary_type(),
            &by_name,
            &mut reachable,
            &mut BTreeSet::new(),
        )?;
        if let Some(domain) = root.domain_type() {
            if !by_name.contains_key(domain) {
                return Err(failure("typed-data domain is unresolved"));
            }
            visit(domain, &by_name, &mut reachable, &mut BTreeSet::new())?;
        }
    }
    if reachable.len() != types.len() {
        return Err(failure("typed-data schema contains unreachable types"));
    }
    Ok(())
}

fn visit<'a>(
    name: &'a str,
    types: &BTreeMap<&'a str, &'a TypedDataStruct>,
    reachable: &mut BTreeSet<&'a str>,
    visiting: &mut BTreeSet<&'a str>,
) -> Result<(), IrError> {
    if reachable.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name) {
        return Err(failure("typed-data schema contains a recursive cycle"));
    }
    for field in &types
        .get(name)
        .ok_or_else(|| failure("typed-data type is unresolved"))?
        .fields
    {
        references(&field.ty, &mut |reference| {
            visit(reference, types, reachable, visiting)
        })?;
    }
    visiting.remove(name);
    reachable.insert(name);
    Ok(())
}

fn references<'a>(
    ty: &'a TypedDataType,
    visit: &mut impl FnMut(&'a str) -> Result<(), IrError>,
) -> Result<(), IrError> {
    match ty {
        TypedDataType::Struct(name) => visit(name),
        TypedDataType::Array(element) | TypedDataType::FixedArray(element, _) => {
            references(element, visit)
        }
        _ => Ok(()),
    }
}

pub(crate) fn identifier(value: &str, limits: HardLimits) -> Result<(), IrError> {
    if value.len() > limits.identifier_bytes {
        return Err(failure("typed-data identifier exceeds the effective limit"));
    }
    crate::names::validate_abi_name(value).map_err(diagnostic)
}

fn diagnostic(value: Diagnostic) -> IrError {
    IrError {
        diagnostics: one(value),
    }
}

pub(crate) fn failure(message: &str) -> IrError {
    diagnostic(Diagnostic::error(DiagnosticCode::InvalidInput, message))
}
