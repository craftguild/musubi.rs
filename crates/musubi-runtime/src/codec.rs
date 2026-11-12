//! Strict owned-value conversion to a private Alloy dynamic ABI implementation.

use alloy_dyn_abi::{DynSolType, DynSolValue};
use alloy_primitives::{
    Address as AlloyAddress, FixedBytes as AlloyFixedBytes, Function as AlloyFunction, I256,
    U256 as AlloyU256,
};
use musubi_core::ABI_FIXED_ARRAY_ELEMENTS_LIMIT;
use musubi_core::{AbiField, AbiType, AbiValue, Address, Bytes, FunctionPointer};

use crate::typed_data_graph::MAX_DEPTH;

/// Strict ABI codec failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CodecError {
    /// Argument count does not match the descriptor.
    #[error("ABI value count does not match descriptor")]
    Arity,
    /// A dynamic value variant or shape does not match its descriptor.
    #[error("ABI value does not match descriptor")]
    TypeMismatch,
    /// Encoded data is malformed or not canonical for the descriptor.
    #[error("ABI data is malformed or non-canonical")]
    Malformed,
    /// Function selector does not match the selected descriptor.
    #[error("function selector does not match descriptor")]
    Selector,
    /// Function mutability does not match the selected operation builder.
    #[error("function state mutability is incompatible with operation kind")]
    OperationKind,
    /// Native value was attached to a function that cannot receive it.
    #[error("native value is only allowed for payable functions")]
    UnexpectedValue,
    /// A function request omitted its contract target.
    #[error("function request requires a target contract address")]
    MissingTarget,
}

/// Encodes a descriptor/value sequence using canonical Solidity ABI encoding.
///
/// # Errors
///
/// Returns [`CodecError`] when arity, width, array length, tuple shape, or value kind differs from
/// the descriptor.
pub fn encode_fields(fields: &[AbiField], values: &[AbiValue]) -> Result<Bytes, CodecError> {
    if fields.len() != values.len() {
        return Err(CodecError::Arity);
    }
    for field in fields {
        validate_type(&field.ty, 0, 1)?;
    }
    let dynamic = fields
        .iter()
        .zip(values)
        .map(|(field, value)| to_alloy(&field.ty, value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Bytes::new(DynSolValue::Tuple(dynamic).abi_encode_params()))
}

/// Strictly decodes a canonical ABI sequence into owned dynamic values.
///
/// # Errors
///
/// Returns [`CodecError::Malformed`] for invalid offsets, padding, trailing bytes, or values that
/// cannot be re-encoded byte-identically.
pub fn decode_fields(fields: &[AbiField], data: &[u8]) -> Result<Vec<AbiValue>, CodecError> {
    for field in fields {
        validate_type(&field.ty, 0, 1)?;
    }
    let tuple_type = DynSolType::Tuple(
        fields
            .iter()
            .map(|field| to_alloy_type(&field.ty))
            .collect(),
    );
    let decoded = tuple_type
        .abi_decode_params(data)
        .map_err(|_| CodecError::Malformed)?;
    if decoded.abi_encode_params() != data {
        return Err(CodecError::Malformed);
    }
    let DynSolValue::Tuple(values) = decoded else {
        return Err(CodecError::Malformed);
    };
    fields
        .iter()
        .zip(values)
        .map(|(field, value)| from_alloy(&field.ty, value).map_err(|_| CodecError::Malformed))
        .collect()
}

pub(crate) fn encode_function(
    selector: [u8; 4],
    fields: &[AbiField],
    values: &[AbiValue],
) -> Result<Bytes, CodecError> {
    let encoded = encode_fields(fields, values)?;
    let mut output = Vec::with_capacity(4 + encoded.len());
    output.extend_from_slice(&selector);
    output.extend_from_slice(encoded.as_ref());
    Ok(Bytes::new(output))
}

pub(crate) fn decode_function_input(
    selector: [u8; 4],
    fields: &[AbiField],
    data: &[u8],
) -> Result<Vec<AbiValue>, CodecError> {
    if data.get(..4) != Some(selector.as_slice()) {
        return Err(CodecError::Selector);
    }
    decode_fields(fields, &data[4..])
}

fn to_alloy_type(ty: &AbiType) -> DynSolType {
    match ty {
        AbiType::Address => DynSolType::Address,
        AbiType::Bool => DynSolType::Bool,
        AbiType::String => DynSolType::String,
        AbiType::Bytes => DynSolType::Bytes,
        AbiType::FixedBytes(size) => DynSolType::FixedBytes(*size),
        AbiType::Uint(bits) | AbiType::UFixed(bits, _) => DynSolType::Uint(*bits),
        AbiType::Int(bits) | AbiType::Fixed(bits, _) => DynSolType::Int(*bits),
        AbiType::Function => DynSolType::Function,
        AbiType::Array(element) => DynSolType::Array(Box::new(to_alloy_type(element))),
        AbiType::FixedArray(element, length) => {
            DynSolType::FixedArray(Box::new(to_alloy_type(element)), *length)
        }
        AbiType::Tuple(fields) => DynSolType::Tuple(
            fields
                .iter()
                .map(|field| to_alloy_type(&field.ty))
                .collect(),
        ),
    }
}

fn to_alloy(ty: &AbiType, value: &AbiValue) -> Result<DynSolValue, CodecError> {
    match (ty, value) {
        (AbiType::Address, AbiValue::Address(value)) => {
            Ok(DynSolValue::Address(AlloyAddress::from(value.into_array())))
        }
        (AbiType::Bool, AbiValue::Bool(value)) => Ok(DynSolValue::Bool(*value)),
        (AbiType::String, AbiValue::String(value)) => Ok(DynSolValue::String(value.clone())),
        (AbiType::Bytes, AbiValue::Bytes(value)) => Ok(DynSolValue::Bytes(value.as_ref().to_vec())),
        (AbiType::FixedBytes(size), AbiValue::FixedBytes(value)) if value.len() == *size => {
            let mut word = [0; 32];
            word[..*size].copy_from_slice(value.as_ref());
            Ok(DynSolValue::FixedBytes(AlloyFixedBytes::from(word), *size))
        }
        (AbiType::Uint(bits), AbiValue::Uint(word))
        | (AbiType::UFixed(bits, _), AbiValue::UFixed(word)) => unsigned(*bits, *word),
        (AbiType::Int(bits), AbiValue::Int(word))
        | (AbiType::Fixed(bits, _), AbiValue::Fixed(word)) => signed(*bits, *word),
        (AbiType::Function, AbiValue::Function(value)) => {
            let mut bytes = [0; 24];
            bytes[..20].copy_from_slice(value.address.as_ref());
            bytes[20..].copy_from_slice(&value.selector);
            Ok(DynSolValue::Function(AlloyFunction::from(bytes)))
        }
        (AbiType::Array(element), AbiValue::Array(values)) => Ok(DynSolValue::Array(
            values
                .iter()
                .map(|value| to_alloy(element, value))
                .collect::<Result<_, _>>()?,
        )),
        (AbiType::FixedArray(element, length), AbiValue::Array(values))
            if values.len() == *length =>
        {
            Ok(DynSolValue::FixedArray(
                values
                    .iter()
                    .map(|value| to_alloy(element, value))
                    .collect::<Result<_, _>>()?,
            ))
        }
        (AbiType::Tuple(fields), AbiValue::Tuple(values)) if fields.len() == values.len() => {
            Ok(DynSolValue::Tuple(
                fields
                    .iter()
                    .zip(values)
                    .map(|(field, value)| to_alloy(&field.ty, value))
                    .collect::<Result<_, _>>()?,
            ))
        }
        _ => Err(CodecError::TypeMismatch),
    }
}

fn unsigned(bits: usize, word: [u8; 32]) -> Result<DynSolValue, CodecError> {
    validate_unsigned(bits, &word)?;
    Ok(DynSolValue::Uint(AlloyU256::from_be_bytes(word), bits))
}

fn signed(bits: usize, word: [u8; 32]) -> Result<DynSolValue, CodecError> {
    validate_signed(bits, &word)?;
    Ok(DynSolValue::Int(
        I256::from_raw(AlloyU256::from_be_bytes(word)),
        bits,
    ))
}

fn from_alloy(ty: &AbiType, value: DynSolValue) -> Result<AbiValue, CodecError> {
    match (ty, value) {
        (AbiType::Address, DynSolValue::Address(value)) => {
            Ok(AbiValue::Address(Address::new(value.into_array())))
        }
        (AbiType::Bool, DynSolValue::Bool(value)) => Ok(AbiValue::Bool(value)),
        (AbiType::String, DynSolValue::String(value)) => Ok(AbiValue::String(value)),
        (AbiType::Bytes, DynSolValue::Bytes(value)) => Ok(AbiValue::Bytes(Bytes::new(value))),
        (AbiType::FixedBytes(size), DynSolValue::FixedBytes(word, actual)) if *size == actual => {
            if word[actual..].iter().any(|byte| *byte != 0) {
                return Err(CodecError::Malformed);
            }
            Ok(AbiValue::FixedBytes(Bytes::new(word[..actual].to_vec())))
        }
        (AbiType::Uint(bits), DynSolValue::Uint(value, actual)) if *bits == actual => {
            let word = value.to_be_bytes();
            validate_unsigned(*bits, &word)?;
            Ok(AbiValue::Uint(word))
        }
        (AbiType::UFixed(bits, _), DynSolValue::Uint(value, actual)) if *bits == actual => {
            let word = value.to_be_bytes();
            validate_unsigned(*bits, &word)?;
            Ok(AbiValue::UFixed(word))
        }
        (AbiType::Int(bits), DynSolValue::Int(value, actual)) if *bits == actual => {
            let word = value.into_raw().to_be_bytes();
            validate_signed(*bits, &word)?;
            Ok(AbiValue::Int(word))
        }
        (AbiType::Fixed(bits, _), DynSolValue::Int(value, actual)) if *bits == actual => {
            let word = value.into_raw().to_be_bytes();
            validate_signed(*bits, &word)?;
            Ok(AbiValue::Fixed(word))
        }
        (AbiType::Function, DynSolValue::Function(value)) => {
            let raw = value.into_array();
            Ok(AbiValue::Function(FunctionPointer {
                address: Address::new(raw[..20].try_into().map_err(|_| CodecError::Malformed)?),
                selector: raw[20..].try_into().map_err(|_| CodecError::Malformed)?,
            }))
        }
        (AbiType::Array(element), DynSolValue::Array(values)) => Ok(AbiValue::Array(
            values
                .into_iter()
                .map(|value| from_alloy(element, value))
                .collect::<Result<_, _>>()?,
        )),
        (AbiType::FixedArray(element, length), DynSolValue::FixedArray(values))
            if values.len() == *length =>
        {
            Ok(AbiValue::Array(
                values
                    .into_iter()
                    .map(|value| from_alloy(element, value))
                    .collect::<Result<_, _>>()?,
            ))
        }
        (AbiType::Tuple(fields), DynSolValue::Tuple(values)) if fields.len() == values.len() => {
            Ok(AbiValue::Tuple(
                fields
                    .iter()
                    .zip(values)
                    .map(|(field, value)| from_alloy(&field.ty, value))
                    .collect::<Result<_, _>>()?,
            ))
        }
        _ => Err(CodecError::TypeMismatch),
    }
}

fn validate_unsigned(bits: usize, word: &[u8; 32]) -> Result<(), CodecError> {
    if !(8..=256).contains(&bits) || !bits.is_multiple_of(8) {
        return Err(CodecError::TypeMismatch);
    }
    let unused = 256_usize
        .checked_sub(bits)
        .ok_or(CodecError::TypeMismatch)?;
    if bits == 0 || word[..unused / 8].iter().any(|byte| *byte != 0) {
        return Err(CodecError::TypeMismatch);
    }
    Ok(())
}

fn validate_signed(bits: usize, word: &[u8; 32]) -> Result<(), CodecError> {
    if !(8..=256).contains(&bits) || !bits.is_multiple_of(8) {
        return Err(CodecError::TypeMismatch);
    }
    let sign_index = 256 - bits;
    let byte = sign_index / 8;
    let bit = 7 - sign_index % 8;
    let negative = word[byte] & (1 << bit) != 0;
    let expected = if negative { 0xff } else { 0 };
    let upper_mask = if bit == 7 {
        0
    } else {
        !((1_u8 << (bit + 1)) - 1)
    };
    if word[..byte].iter().any(|value| *value != expected)
        || word[byte] & upper_mask != expected & upper_mask
    {
        return Err(CodecError::TypeMismatch);
    }
    Ok(())
}

pub(crate) fn validate_type(
    ty: &AbiType,
    depth: usize,
    fixed_elements: usize,
) -> Result<(), CodecError> {
    if depth > MAX_DEPTH {
        return Err(CodecError::TypeMismatch);
    }
    let nested_fixed_elements = match ty {
        AbiType::FixedArray(_, length) if *length != 0 => fixed_elements
            .checked_mul(*length)
            .ok_or(CodecError::TypeMismatch)?,
        AbiType::FixedArray(_, _) => return Err(CodecError::TypeMismatch),
        _ => fixed_elements,
    };
    if nested_fixed_elements > ABI_FIXED_ARRAY_ELEMENTS_LIMIT {
        return Err(CodecError::TypeMismatch);
    }
    match ty {
        AbiType::FixedBytes(size) if !(1..=32).contains(size) => Err(CodecError::TypeMismatch),
        AbiType::Uint(bits) | AbiType::Int(bits) if !(8..=256).contains(bits) || *bits % 8 != 0 => {
            Err(CodecError::TypeMismatch)
        }
        AbiType::UFixed(bits, decimals) | AbiType::Fixed(bits, decimals)
            if !(8..=256).contains(bits) || *bits % 8 != 0 || *decimals > 80 =>
        {
            Err(CodecError::TypeMismatch)
        }
        AbiType::Array(element) => validate_type(element, depth + 1, fixed_elements),
        AbiType::FixedArray(element, _) => validate_type(element, depth + 1, nested_fixed_elements),
        AbiType::Tuple(fields) => fields
            .iter()
            .try_for_each(|field| validate_type(&field.ty, depth + 1, fixed_elements)),
        _ => Ok(()),
    }
}
