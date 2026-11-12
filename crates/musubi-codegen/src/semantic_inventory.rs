//! Bounded semantic path expectations shared with source-input inspection consumers.

use crate::{HardLimits, RenderError, SemanticOwner, render::render_error, render_layout::Naming};

/// Derives the sole private path and module spelling for a semantic owner.
///
/// The caller supplies validated identity bytes: canonical wire signature, named/owned type
/// identity, or typed-data authority and type identity. This names an owner; it does not prove its
/// provenance. Whole-tree ownership validation additionally rejects collisions across owners.
///
/// # Errors
/// Rejects unknown owner kinds, non-Solidity identifiers, and names or identity bytes exceeding
/// hard input/output bounds. Names must use the ABI spelling, without Rust raw-identifier syntax.
pub fn semantic_owner(
    kind: &str,
    name: &str,
    identity: &[u8],
) -> Result<SemanticOwner, RenderError> {
    if !matches!(
        kind,
        "operations" | "events" | "errors" | "shared_types" | "types"
    ) || name.len() > HardLimits::V1.identifier_bytes
        || identity.len() > HardLimits::V1.generated_bytes
    {
        return Err(render_error("semantic owner identity is invalid"));
    }
    crate::names::validate_abi_name(name)
        .map_err(|_| render_error("semantic owner name is invalid"))?;
    Naming::default().owner(kind, name, identity)
}

/// Derives operation/event/error module expectations from a retained standard ABI asset.
///
/// Uses the generator's ABI parser exactly once per contract, without AST enrichment, rendering,
/// I/O, or Cargo execution. Asset size is charged to the generated-output bound because canonical
/// pretty-printing can make the generated ABI larger than its original input.
/// A streaming structural preflight enforces item/component budgets before JSON tree allocation.
///
/// # Errors
/// Rejects invalid ABI shape, ambiguous signatures, names, recursive types, and resource limits.
pub fn semantic_abi_modules(bytes: &[u8]) -> Result<Vec<SemanticOwner>, RenderError> {
    if bytes.len() > HardLimits::V1.generated_bytes {
        return Err(RenderError {
            diagnostic: crate::Diagnostic::error(
                crate::DiagnosticCode::LimitExceeded,
                "ABI asset exceeds output limits",
            ),
        });
    }
    let values = crate::abi_json::parse(bytes, HardLimits::V1)
        .map_err(|diagnostic| RenderError { diagnostic })?;
    let abi = crate::abi_parse::parse_abi(&values, HardLimits::V1)
        .map_err(|diagnostic| RenderError { diagnostic })?;
    let mut names = Naming::default();
    let mut output = Vec::new();
    for function in abi.functions {
        output.push(names.owner("operations", &function.name, function.signature.as_bytes())?);
    }
    for event in abi.events {
        output.push(names.owner("events", &event.name, event.signature.as_bytes())?);
    }
    for error in abi.errors {
        output.push(names.owner("errors", &error.name, error.signature.as_bytes())?);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    //! The public naming boundary rejects malformed input before byte-indexed normalization.

    use super::semantic_owner;

    #[test]
    fn public_semantic_owner_rejects_noncanonical_names_without_panicking() {
        for name in [
            "",
            "../escape",
            "/root",
            "a\\b",
            "1name",
            "a\n",
            "r#type",
            "é",
        ] {
            assert!(semantic_owner("types", name, b"identity").is_err());
        }
        assert!(semantic_owner("types", &format!("{}é", "a".repeat(47)), b"x").is_err());
        for name in ["type", "$name", "_", "ValidName"] {
            let owner = semantic_owner("types", name, b"identity").unwrap();
            assert!(owner.path.is_ascii());
            assert!(owner.module.is_ascii());
            assert_eq!(owner.path.split('/').count(), 2);
        }
    }
}
