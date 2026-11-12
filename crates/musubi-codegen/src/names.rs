//! Solidity and Rust identifier validation and deterministic normalization.

use std::collections::BTreeSet;

use musubi_core::TypedDataRoot;

use crate::{Diagnostic, DiagnosticCode};

pub(crate) fn validate_abi_name(name: &str) -> Result<(), Diagnostic> {
    let mut chars = name.chars();
    let first = chars.next().ok_or_else(invalid_name)?;
    if !(matches!(first, '_' | '$') || first.is_ascii_alphabetic())
        || chars
            .any(|character| !(matches!(character, '_' | '$') || character.is_ascii_alphanumeric()))
    {
        return Err(invalid_name());
    }
    Ok(())
}

pub(crate) fn module_name(configured: Option<&str>, id: &str) -> Result<String, Diagnostic> {
    let value = configured.unwrap_or(id);
    validate_abi_name(value)?;
    Ok(rust_module(value))
}

/// Produces a source-safe Rust value or field identifier.
pub(crate) fn rust_snake(name: &str) -> String {
    escape_snake(&snake_base(name), "value")
}

/// Produces a snake-case fragment that is safe after a non-keyword identifier prefix.
pub(crate) fn rust_snake_fragment(name: &str) -> String {
    snake_base(name)
}

pub(crate) fn typed_data_digest_method(
    root: &TypedDataRoot,
    roots: &[TypedDataRoot],
) -> Option<String> {
    let domain = root.domain_type()?;
    let root_count = roots
        .iter()
        .filter(|candidate| candidate.primary_type() == root.primary_type())
        .count();
    Some(if root_count == 1 {
        "digest".into()
    } else {
        format!("digest_with_{}", rust_snake_fragment(domain))
    })
}

/// Produces a source-safe Rust module identifier.
pub(crate) fn rust_module(name: &str) -> String {
    let name = snake_base(name);
    if name == "_" {
        "underscore_contract".to_owned()
    } else if is_keyword(&name) {
        format!("{name}_contract")
    } else {
        name
    }
}

fn snake_base(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    for (index, character) in name.chars().enumerate() {
        if character == '$' {
            if !output.is_empty() && !output.ends_with('_') {
                output.push('_');
            }
            output.push_str("dollar_");
        } else if character.is_ascii_uppercase() {
            if index > 0 && !output.ends_with('_') {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character.to_ascii_lowercase());
        }
    }
    output
}

/// Produces a source-safe Rust type identifier.
pub(crate) fn rust_pascal(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    let mut uppercase = true;
    for character in name.chars() {
        if character == '$' {
            output.push_str("Dollar");
            uppercase = true;
        } else if character == '_' {
            uppercase = true;
        } else if uppercase {
            output.push(character.to_ascii_uppercase());
            uppercase = false;
        } else {
            output.push(character);
        }
    }
    match output.as_str() {
        "" => "Anonymous".into(),
        "Self" => "SelfType".into(),
        _ => output,
    }
}

pub(crate) fn reserved_typed_data_type_names() -> BTreeSet<String> {
    [
        "AbiValue",
        "Address",
        "Box",
        "Bytes",
        "FixedBytes",
        "Hash",
        "Int",
        "Result",
        "String",
        "TypedDataError",
        "Uint",
        "Vec",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// Produces a source-safe Rust enum variant identifier.
pub(crate) fn rust_variant(name: &str) -> String {
    let value = rust_pascal(name);
    if value == "SelfType" && matches!(name, "self" | "Self") {
        "SelfVariant".into()
    } else {
        value
    }
}

/// Produces the field identifier used by generated struct declarations.
pub(crate) fn rust_field(name: &str, index: usize) -> String {
    if name.is_empty() {
        format!("value_{index}")
    } else {
        rust_snake(name)
    }
}

fn invalid_name() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "ABI name is not a valid Solidity identifier",
    )
}

fn escape_snake(name: &str, non_raw_suffix: &str) -> String {
    if name == "_" {
        format!("underscore_{non_raw_suffix}")
    } else if is_non_raw_keyword(name) {
        format!("{name}_{non_raw_suffix}")
    } else if is_keyword(name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

fn is_non_raw_keyword(name: &str) -> bool {
    matches!(name, "_" | "crate" | "self" | "super" | "Self")
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "_" | "as"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "gen"
    )
}
