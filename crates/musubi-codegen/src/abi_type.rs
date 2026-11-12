//! Recursive ABI field type grammar and canonical type spelling.

use musubi_core::{AbiField, AbiType};
use serde::Deserialize;

use crate::{Diagnostic, DiagnosticCode, HardLimits, names::validate_abi_name};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct RawField {
    #[serde(default)]
    name: String,
    #[serde(rename = "type")]
    ty: String,
    internal_type: Option<String>,
    #[serde(default)]
    components: Vec<Self>,
    indexed: Option<bool>,
}

impl RawField {
    pub(crate) const fn indexed(&self) -> Option<bool> {
        self.indexed
    }
}

pub(crate) fn fields(
    raw: &[RawField],
    limits: HardLimits,
    count: &mut usize,
    indexed_allowed: bool,
) -> Result<Vec<AbiField>, Diagnostic> {
    raw.iter()
        .map(|field| {
            if field.name.len() > limits.identifier_bytes
                || (!indexed_allowed && field.indexed.is_some())
                || (!field.name.is_empty() && validate_abi_name(&field.name).is_err())
            {
                return Err(invalid_type());
            }
            increment(count, limits)?;
            Ok(AbiField {
                name: field.name.clone(),
                ty: parse_type(field, limits, 0, count, 1)?,
                internal_type: field.internal_type.clone(),
            })
        })
        .collect()
}

fn parse_type(
    raw: &RawField,
    limits: HardLimits,
    depth: usize,
    count: &mut usize,
    parent_fixed_elements: usize,
) -> Result<AbiType, Diagnostic> {
    if depth > limits.nesting {
        return Err(limit_error());
    }
    let remaining = limits.nesting.checked_sub(depth).ok_or_else(limit_error)?;
    let (base, dimensions) = split_arrays(&raw.ty, remaining)?;
    if base != "tuple" && !raw.components.is_empty() {
        return Err(invalid_type());
    }
    let fixed_elements = dimensions
        .iter()
        .flatten()
        .try_fold(parent_fixed_elements, |product, length| {
            product.checked_mul(*length).ok_or_else(limit_error)
        })?;
    if fixed_elements > limits.abi_fixed_array_elements {
        return Err(limit_error());
    }
    let wrapped_depth = depth
        .checked_add(dimensions.len())
        .ok_or_else(limit_error)?;
    let mut ty = match base {
        "address" => AbiType::Address,
        "bool" => AbiType::Bool,
        "string" => AbiType::String,
        "bytes" => AbiType::Bytes,
        "function" => AbiType::Function,
        "tuple" => {
            let child_depth = wrapped_depth.checked_add(1).ok_or_else(limit_error)?;
            if child_depth > limits.nesting {
                return Err(limit_error());
            }
            AbiType::Tuple(
                raw.components
                    .iter()
                    .map(|component| {
                        if component.name.len() > limits.identifier_bytes
                            || component.indexed.is_some()
                            || (!component.name.is_empty()
                                && validate_abi_name(&component.name).is_err())
                        {
                            return Err(invalid_type());
                        }
                        increment(count, limits)?;
                        Ok(AbiField {
                            name: component.name.clone(),
                            ty: parse_type(component, limits, child_depth, count, fixed_elements)?,
                            internal_type: component.internal_type.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            )
        }
        value if value.starts_with("bytes") => {
            AbiType::FixedBytes(parse_number(&value[5..], 1, 32, false)?)
        }
        value if value.starts_with("uint") => {
            AbiType::Uint(parse_number(&value[4..], 8, 256, true)?)
        }
        value if value.starts_with("int") => AbiType::Int(parse_number(&value[3..], 8, 256, true)?),
        value if value.starts_with("ufixed") => {
            let (bits, decimals) = fixed_parts(&value[6..])?;
            AbiType::UFixed(bits, decimals)
        }
        value if value.starts_with("fixed") => {
            let (bits, decimals) = fixed_parts(&value[5..])?;
            AbiType::Fixed(bits, decimals)
        }
        _ => return Err(invalid_type()),
    };
    for dimension in dimensions {
        ty = match dimension {
            Some(length) => AbiType::FixedArray(Box::new(ty), length),
            None => AbiType::Array(Box::new(ty)),
        };
    }
    Ok(ty)
}

fn split_arrays(
    value: &str,
    maximum_dimensions: usize,
) -> Result<(&str, Vec<Option<usize>>), Diagnostic> {
    let base_end = value.find('[').unwrap_or(value.len());
    let mut rest = &value[base_end..];
    let mut dimensions = Vec::new();
    while !rest.is_empty() {
        if !rest.starts_with('[') {
            return Err(invalid_type());
        }
        let end = rest.find(']').ok_or_else(invalid_type)?;
        let length = if end == 1 {
            None
        } else {
            Some(parse_decimal(&rest[1..end])?)
        };
        if length == Some(0) {
            return Err(invalid_type());
        }
        dimensions.push(length);
        if dimensions.len() > maximum_dimensions {
            return Err(limit_error());
        }
        rest = &rest[end + 1..];
    }
    Ok((&value[..base_end], dimensions))
}

fn parse_number(
    value: &str,
    minimum: usize,
    maximum: usize,
    multiple_eight: bool,
) -> Result<usize, Diagnostic> {
    let number = if value.is_empty() && maximum == 256 {
        256
    } else {
        parse_decimal(value)?
    };
    if !(minimum..=maximum).contains(&number) || (multiple_eight && number % 8 != 0) {
        return Err(invalid_type());
    }
    Ok(number)
}

fn parse_decimal(value: &str) -> Result<usize, Diagnostic> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(invalid_type());
    }
    value.parse().map_err(|_| invalid_type())
}

fn fixed_parts(value: &str) -> Result<(usize, usize), Diagnostic> {
    if value.is_empty() {
        return Ok((128, 18));
    }
    let (bits, decimals) = value.split_once('x').ok_or_else(invalid_type)?;
    Ok((
        parse_number(bits, 8, 256, true)?,
        parse_number(decimals, 0, 80, false)?,
    ))
}

pub(crate) fn canonical_type(ty: &AbiType) -> String {
    match ty {
        AbiType::Address => "address".into(),
        AbiType::Bool => "bool".into(),
        AbiType::String => "string".into(),
        AbiType::Bytes => "bytes".into(),
        AbiType::FixedBytes(size) => format!("bytes{size}"),
        AbiType::Uint(bits) => format!("uint{bits}"),
        AbiType::Int(bits) => format!("int{bits}"),
        AbiType::UFixed(bits, decimals) => format!("ufixed{bits}x{decimals}"),
        AbiType::Fixed(bits, decimals) => format!("fixed{bits}x{decimals}"),
        AbiType::Function => "function".into(),
        AbiType::Array(element) => format!("{}[]", canonical_type(element)),
        AbiType::FixedArray(element, length) => format!("{}[{length}]", canonical_type(element)),
        AbiType::Tuple(fields) => format!(
            "({})",
            fields
                .iter()
                .map(|field| canonical_type(&field.ty))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn invalid_type() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "ABI field has an invalid wire type",
    )
}

fn limit_error() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::LimitExceeded,
        "ABI nesting or component count exceeds the effective limit",
    )
}

fn increment(count: &mut usize, limits: HardLimits) -> Result<(), Diagnostic> {
    *count = count.checked_add(1).ok_or_else(limit_error)?;
    if *count > limits.abi_components {
        Err(limit_error())
    } else {
        Ok(())
    }
}
