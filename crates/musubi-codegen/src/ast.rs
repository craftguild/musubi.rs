//! Optional solc `SourceUnit` enrichment that cannot alter ABI wire descriptors.

use musubi_core::{AbiField, AbiType, ContractAbi};

use crate::{
    Diagnostic, DiagnosticCode, Diagnostics, Enrichment, EnrichmentKind, HardLimits,
    ast_internal::{internal_type_occurrences, internal_wrappers_match, split_internal, wire_base},
    ast_type::{Candidate, CandidateKind, Candidates, Comparison, compare, valid_qualified},
    ast_type_parse::{TypeKind, parse as parse_type_string},
    names::{rust_pascal, validate_abi_name},
};

pub(crate) fn enrich(
    abi: &ContractAbi,
    ast: Option<&serde_json::Value>,
    limits: HardLimits,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<Enrichment>, Diagnostic> {
    let Some(ast) = ast else {
        return Ok(Vec::new());
    };
    if ast.get("nodeType").and_then(serde_json::Value::as_str) != Some("SourceUnit") {
        return Err(inconsistent("AST root is not a solc SourceUnit"));
    }
    let mut candidates = Vec::new();
    let mut node_count = 0_usize;
    walk(ast, &mut node_count, limits, &mut candidates)?;
    let candidates = Candidates::new(candidates);
    let occurrences = internal_type_occurrences(abi, limits.nesting);
    let mut enrichments = Vec::new();
    for (internal, matching_fields) in occurrences {
        let Some((prefix, identity)) =
            split_internal(&internal, limits.identifier_bytes, limits.nesting)
        else {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::AstEnrichmentUnavailable,
                "AST enrichment identity is malformed; ABI type retained",
            ));
            continue;
        };
        let candidate_kind = match prefix {
            "struct" => TypeKind::Struct,
            "enum" => TypeKind::Enum,
            "type" => TypeKind::Udvt,
            _ => unreachable!("split_internal returns a known prefix"),
        };
        let Some(candidate) = candidates.unique(candidate_kind, identity) else {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::AstEnrichmentUnavailable,
                format!(
                    "AST enrichment for `{internal}` is missing or ambiguous; ABI type retained"
                ),
            ));
            continue;
        };
        let Some(kind) = validate_candidate(candidate, &matching_fields, &candidates, limits)?
        else {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::AstEnrichmentUnavailable,
                format!(
                    "AST enrichment for `{internal}` contains an unsupported type; ABI type retained"
                ),
            ));
            continue;
        };
        enrichments.push(Enrichment {
            internal_type: internal,
            rust_name: rust_pascal(&candidate.name),
            kind,
        });
    }
    enrichments.sort_by(|left, right| left.internal_type.cmp(&right.internal_type));
    Ok(enrichments)
}

fn walk(
    value: &serde_json::Value,
    count: &mut usize,
    limits: HardLimits,
    output: &mut Vec<Candidate>,
) -> Result<(), Diagnostic> {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(node_type) = object.get("nodeType").and_then(serde_json::Value::as_str) {
                *count = count.checked_add(1).ok_or_else(limit_error)?;
                if *count > limits.ast_nodes {
                    return Err(limit_error());
                }
                if matches!(
                    node_type,
                    "EnumDefinition" | "UserDefinedValueTypeDefinition" | "StructDefinition"
                ) {
                    output.push(parse_candidate(node_type, object, limits)?);
                }
            }
            for child in object.values() {
                walk(child, count, limits, output)?;
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                walk(child, count, limits, output)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_candidate(
    node_type: &str,
    object: &serde_json::Map<String, serde_json::Value>,
    limits: HardLimits,
) -> Result<Candidate, Diagnostic> {
    let name = string(object, "name")?.to_owned();
    validate_ast_name(&name, limits.identifier_bytes)?;
    let canonical_name = object
        .get("canonicalName")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if canonical_name
        .as_deref()
        .is_some_and(|name| !valid_qualified(name, limits.identifier_bytes))
    {
        return Err(inconsistent(
            "AST definition has an invalid canonical identity",
        ));
    }
    let kind = match node_type {
        "EnumDefinition" => CandidateKind::Enum(
            array(object, "members")?
                .iter()
                .map(|member| {
                    let name = member
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| inconsistent("enum member has no name"))?;
                    validate_ast_name(name, limits.identifier_bytes)?;
                    Ok(name.to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
        "UserDefinedValueTypeDefinition" => CandidateKind::Udvt(parse_type_string(
            type_string(
                object
                    .get("underlyingType")
                    .ok_or_else(|| inconsistent("UDVT has no underlying type"))?,
            )?,
            limits.nesting,
            limits.identifier_bytes,
        )),
        "StructDefinition" => CandidateKind::Struct(
            array(object, "members")?
                .iter()
                .map(|member| {
                    let member_object = member
                        .as_object()
                        .ok_or_else(|| inconsistent("struct member is not an object"))?;
                    let name = string(member_object, "name")?;
                    validate_ast_name(name, limits.identifier_bytes)?;
                    Ok((
                        name.to_owned(),
                        parse_type_string(
                            type_string(member)?,
                            limits.nesting,
                            limits.identifier_bytes,
                        ),
                    ))
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        ),
        _ => return Err(inconsistent("unsupported enrichment definition")),
    };
    Ok(Candidate {
        name,
        canonical_name,
        kind,
    })
}

fn validate_candidate(
    candidate: &Candidate,
    matching_fields: &[&AbiField],
    candidates: &Candidates,
    limits: HardLimits,
) -> Result<Option<EnrichmentKind>, Diagnostic> {
    if matching_fields
        .iter()
        .any(|field| !internal_wrappers_match(field, limits.nesting))
    {
        return Err(inconsistent(
            "AST internal type array wrapper contradicts ABI",
        ));
    }
    let Some(reference) = matching_fields.first() else {
        return Err(inconsistent("AST enrichment has no ABI occurrence"));
    };
    let reference_base = wire_base(&reference.ty);
    if matching_fields
        .iter()
        .skip(1)
        .any(|field| wire_base(&field.ty) != reference_base)
    {
        return Err(inconsistent(
            "ABI occurrences for one AST identity have different structures",
        ));
    }
    match &candidate.kind {
        CandidateKind::Enum(variants) => {
            if variants.is_empty() || variants.len() > 256 || reference_base != &AbiType::Uint(8) {
                return Err(inconsistent("enum AST underlying type contradicts ABI"));
            }
            Ok(Some(EnrichmentKind::Enum {
                variants: variants.clone(),
                underlying: AbiType::Uint(8),
            }))
        }
        CandidateKind::Udvt(parsed) => {
            match compare(parsed, reference_base, candidates, limits.nesting) {
                Comparison::Equal => {}
                Comparison::Unknown => return Ok(None),
                Comparison::Mismatch => {
                    return Err(inconsistent("UDVT AST underlying type contradicts ABI"));
                }
            }
            let underlying = reference_base.clone();
            Ok(Some(EnrichmentKind::UserDefinedValueType { underlying }))
        }
        CandidateKind::Struct(members) => {
            let AbiType::Tuple(fields) = reference_base else {
                return Err(inconsistent("struct AST shape contradicts ABI tuple"));
            };
            if fields.len() != members.len() {
                return Err(inconsistent("struct AST shape contradicts ABI tuple"));
            }
            let mut unknown = false;
            for (field, (name, parsed)) in fields.iter().zip(members) {
                if !field.name.is_empty() && field.name != *name {
                    return Err(inconsistent("struct AST member name contradicts ABI"));
                }
                match compare(parsed, &field.ty, candidates, limits.nesting) {
                    Comparison::Equal => {}
                    Comparison::Unknown => unknown = true,
                    Comparison::Mismatch => {
                        return Err(inconsistent("struct AST member type contradicts ABI"));
                    }
                }
            }
            if unknown {
                return Ok(None);
            }
            Ok(Some(EnrichmentKind::Struct {
                fields: fields.clone(),
            }))
        }
    }
}

fn type_string(value: &serde_json::Value) -> Result<&str, Diagnostic> {
    value
        .get("typeDescriptions")
        .and_then(|value| value.get("typeString"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| inconsistent("AST node has no type description"))
}
fn string<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a str, Diagnostic> {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| inconsistent("AST definition is missing a string field"))
}
fn array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a Vec<serde_json::Value>, Diagnostic> {
    object
        .get(key)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| inconsistent("AST definition is missing members"))
}
fn inconsistent(message: &str) -> Diagnostic {
    Diagnostic::error(DiagnosticCode::InconsistentArtifact, message)
}
fn limit_error() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::LimitExceeded,
        "AST node count exceeds the effective limit",
    )
}

fn validate_ast_name(name: &str, limit: usize) -> Result<(), Diagnostic> {
    if name.len() > limit || validate_abi_name(name).is_err() {
        Err(inconsistent("AST definition has an invalid identifier"))
    } else {
        Ok(())
    }
}
