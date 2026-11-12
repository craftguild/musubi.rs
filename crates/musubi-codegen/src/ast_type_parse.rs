//! One-time bounded normalization of solc AST type strings.

use musubi_core::AbiType;

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static PARSE_CALLS: Cell<usize> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum TypeKind {
    Struct,
    Enum,
    Udvt,
}

#[derive(Clone)]
pub(super) struct ParsedType {
    pub(super) base: ParsedBase,
    pub(super) dimensions: Vec<Option<usize>>,
    pub(super) nesting: usize,
}

#[derive(Clone)]
pub(super) enum ParsedBase {
    Abi(AbiType),
    InternalFunction,
    Named {
        kind: Option<TypeKind>,
        identity: String,
    },
    Resolved(usize),
    Unknown,
}

pub(super) fn parse(value: &str, maximum_nesting: usize, identifier_limit: usize) -> ParsedType {
    #[cfg(test)]
    PARSE_CALLS.set(PARSE_CALLS.get().saturating_add(1));

    let normalized = strip_location(value.trim());
    let Some((base, dimensions)) = split_arrays(normalized, maximum_nesting) else {
        return unknown();
    };
    let remaining = maximum_nesting.saturating_sub(dimensions.len());
    let (base, base_nesting) = parse_base(base, remaining, identifier_limit);
    ParsedType {
        base,
        nesting: dimensions.len().saturating_add(base_nesting),
        dimensions,
    }
}

fn parse_base(value: &str, maximum_nesting: usize, identifier_limit: usize) -> (ParsedBase, usize) {
    if let Some(primitive) = primitive(value) {
        return (ParsedBase::Abi(primitive), 0);
    }
    if let Some(function) = function_syntax(value, maximum_nesting, identifier_limit) {
        return (
            if function.internal {
                ParsedBase::InternalFunction
            } else {
                ParsedBase::Abi(AbiType::Function)
            },
            function.nesting,
        );
    }
    if let Some(identity) = value.strip_prefix("contract ") {
        return (
            if valid_qualified(identity, identifier_limit) {
                ParsedBase::Abi(AbiType::Address)
            } else {
                ParsedBase::Unknown
            },
            0,
        );
    }
    for (prefix, kind) in [
        ("struct ", TypeKind::Struct),
        ("enum ", TypeKind::Enum),
        ("type ", TypeKind::Udvt),
    ] {
        if let Some(identity) = value.strip_prefix(prefix) {
            return (named(identity, Some(kind), identifier_limit), 0);
        }
    }
    (named(value, None, identifier_limit), 0)
}

fn named(value: &str, kind: Option<TypeKind>, identifier_limit: usize) -> ParsedBase {
    if valid_qualified(value, identifier_limit) {
        ParsedBase::Named {
            kind,
            identity: value.to_owned(),
        }
    } else {
        ParsedBase::Unknown
    }
}

fn primitive(value: &str) -> Option<AbiType> {
    match value {
        "address" | "address payable" | "address_payable" => Some(AbiType::Address),
        "bool" => Some(AbiType::Bool),
        "string" => Some(AbiType::String),
        "bytes" => Some(AbiType::Bytes),
        "function" => Some(AbiType::Function),
        _ if value.starts_with("uint") => integer(&value[4..], true),
        _ if value.starts_with("int") => integer(&value[3..], false),
        _ if value.starts_with("bytes") => {
            strict_number(&value[5..], 1, 32, false).map(AbiType::FixedBytes)
        }
        _ if value.starts_with("ufixed") => fixed(&value[6..], true),
        _ if value.starts_with("fixed") => fixed(&value[5..], false),
        _ => None,
    }
}

fn integer(suffix: &str, unsigned: bool) -> Option<AbiType> {
    let bits = if suffix.is_empty() {
        256
    } else {
        strict_number(suffix, 8, 256, true)?
    };
    Some(if unsigned {
        AbiType::Uint(bits)
    } else {
        AbiType::Int(bits)
    })
}

fn fixed(suffix: &str, unsigned: bool) -> Option<AbiType> {
    let (bits, decimals) = if suffix.is_empty() {
        (128, 18)
    } else {
        let (bits, decimals) = suffix.split_once('x')?;
        (
            strict_number(bits, 8, 256, true)?,
            strict_number(decimals, 0, 80, false)?,
        )
    };
    Some(if unsigned {
        AbiType::UFixed(bits, decimals)
    } else {
        AbiType::Fixed(bits, decimals)
    })
}

fn strict_number(
    value: &str,
    minimum: usize,
    maximum: usize,
    multiple_eight: bool,
) -> Option<usize> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    let number = value.parse::<usize>().ok()?;
    ((minimum..=maximum).contains(&number) && (!multiple_eight || number % 8 == 0))
        .then_some(number)
}

struct FunctionSyntax {
    internal: bool,
    nesting: usize,
}

fn function_syntax(
    value: &str,
    maximum_nesting: usize,
    identifier_limit: usize,
) -> Option<FunctionSyntax> {
    if maximum_nesting == 0
        || !value.starts_with("function (")
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return None;
    }
    let end = closing_parenthesis(value, "function ".len())?;
    let suffix = value[end + 1..].trim();
    let (qualifiers, returns) = suffix
        .split_once(" returns ")
        .map_or((suffix, None), |(left, right)| (left, Some(right)));
    let mut words = qualifiers.split_whitespace();
    let visibility = words.next()?;
    if !matches!(visibility, "external" | "internal")
        || words
            .next()
            .is_some_and(|word| !matches!(word, "pure" | "view" | "payable"))
        || words.next().is_some()
    {
        return None;
    }
    let child_limit = maximum_nesting - 1;
    let parameter_nesting = type_list_nesting(
        &value["function (".len()..end],
        child_limit,
        identifier_limit,
    )?;
    let return_nesting = returns.map_or(Some(0), |returns| {
        if !balanced_parenthesized(returns) {
            return None;
        }
        type_list_nesting(
            &returns[1..returns.len().saturating_sub(1)],
            child_limit,
            identifier_limit,
        )
    })?;
    Some(FunctionSyntax {
        internal: visibility == "internal",
        nesting: 1 + parameter_nesting.max(return_nesting),
    })
}

fn closing_parenthesis(value: &str, start: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (index, byte) in value.bytes().enumerate().skip(start) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn type_list_nesting(value: &str, maximum: usize, identifier_limit: usize) -> Option<usize> {
    if value.trim().is_empty() {
        return Some(0);
    }
    let mut depth = 0_i32;
    let mut start = 0;
    let mut nesting = 0;
    for (index, byte) in value.bytes().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                nesting = nesting.max(type_syntax_nesting(
                    value[start..index].trim(),
                    maximum,
                    identifier_limit,
                )?);
                start = index + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    if depth != 0 {
        return None;
    }
    Some(nesting.max(type_syntax_nesting(
        value[start..].trim(),
        maximum,
        identifier_limit,
    )?))
}

fn type_syntax_nesting(value: &str, maximum: usize, identifier_limit: usize) -> Option<usize> {
    let value = strip_location(value);
    let (base, dimensions) = split_arrays(value, maximum)?;
    let remaining = maximum.checked_sub(dimensions.len())?;
    let base_nesting = if primitive(base).is_some() {
        0
    } else if let Some(function) = function_syntax(base, remaining, identifier_limit) {
        function.nesting
    } else if ["contract ", "struct ", "enum ", "type "]
        .into_iter()
        .any(|prefix| {
            base.strip_prefix(prefix)
                .is_some_and(|identity| valid_qualified(identity, identifier_limit))
        })
        || valid_qualified(base, identifier_limit)
    {
        0
    } else {
        return None;
    };
    let nesting = dimensions.len().checked_add(base_nesting)?;
    (nesting <= maximum).then_some(nesting)
}

fn balanced_parenthesized(value: &str) -> bool {
    value.starts_with('(')
        && value.ends_with(')')
        && closing_parenthesis(value, 0) == Some(value.len().saturating_sub(1))
}

pub(super) fn valid_qualified(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|byte| byte == b'_' || byte == b'$' || byte.is_ascii_alphabetic())
                && bytes.all(|byte| byte == b'_' || byte == b'$' || byte.is_ascii_alphanumeric())
        })
}

fn strip_location(mut value: &str) -> &str {
    for suffix in [
        " storage pointer",
        " storage ref",
        " storage",
        " memory",
        " calldata",
    ] {
        if let Some(stripped) = value.strip_suffix(suffix) {
            value = stripped.trim_end();
            break;
        }
    }
    value
}

fn split_arrays(value: &str, maximum: usize) -> Option<(&str, Vec<Option<usize>>)> {
    let mut base = value.trim_end();
    let mut dimensions = Vec::new();
    while base.ends_with(']') {
        let start = base.rfind('[')?;
        if start > 0 && base.as_bytes()[start - 1].is_ascii_whitespace() {
            return None;
        }
        let length = &base[start + 1..base.len() - 1];
        dimensions.push(if length.is_empty() {
            None
        } else {
            Some(strict_number(length, 1, usize::MAX, false)?)
        });
        if dimensions.len() > maximum {
            return None;
        }
        base = base[..start].trim_end();
    }
    Some((base, dimensions))
}

fn unknown() -> ParsedType {
    ParsedType {
        base: ParsedBase::Unknown,
        dimensions: Vec::new(),
        nesting: 0,
    }
}

#[cfg(test)]
pub(super) fn reset_parse_calls() {
    PARSE_CALLS.set(0);
}

#[cfg(test)]
pub(super) fn parse_calls() -> usize {
    PARSE_CALLS.get()
}

#[cfg(test)]
mod tests {
    use musubi_core::AbiType;

    use super::primitive;

    #[test]
    fn fixed_ast_types_accept_explicit_zero_decimals() {
        assert_eq!(primitive("ufixed8x0"), Some(AbiType::UFixed(8, 0)));
        assert_eq!(primitive("fixed256x0"), Some(AbiType::Fixed(256, 0)));
        assert_eq!(primitive("fixed256x81"), None);
    }
}
