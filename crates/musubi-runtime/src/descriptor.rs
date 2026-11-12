//! Canonical validation shared by public ABI descriptor entry points.

use std::fmt::Write as _;

use musubi_core::{AbiField, AbiType, FunctionDescriptor, keccak256};

use crate::{CodecError, codec::validate_type};

pub(crate) fn canonical_signature(name: &str, fields: &[AbiField]) -> Result<String, CodecError> {
    if !valid_identifier(name) {
        return Err(CodecError::Selector);
    }
    validate_fields(fields)?;
    let mut signature = String::from(name);
    signature.push('(');
    for (index, field) in fields.iter().enumerate() {
        validate_type(&field.ty, 0, 1)?;
        if index != 0 {
            signature.push(',');
        }
        canonical_type(&field.ty, &mut signature)?;
    }
    signature.push(')');
    Ok(signature)
}

pub(crate) fn validate_function_descriptor(
    descriptor: &FunctionDescriptor,
) -> Result<(), CodecError> {
    let signature = canonical_signature(&descriptor.name, &descriptor.inputs)?;
    validate_fields(&descriptor.outputs)?;
    let selector: [u8; 4] = keccak256(signature.as_bytes()).as_ref()[..4]
        .try_into()
        .map_err(|_| CodecError::Selector)?;
    if descriptor.signature != signature || descriptor.selector != selector {
        return Err(CodecError::Selector);
    }
    Ok(())
}

fn validate_fields(fields: &[AbiField]) -> Result<(), CodecError> {
    if fields
        .iter()
        .any(|field| !field.name.is_empty() && !valid_identifier(&field.name))
    {
        return Err(CodecError::Selector);
    }
    for field in fields {
        validate_type(&field.ty, 0, 1)?;
    }
    Ok(())
}

fn canonical_type(ty: &AbiType, output: &mut String) -> Result<(), CodecError> {
    match ty {
        AbiType::Address => output.push_str("address"),
        AbiType::Bool => output.push_str("bool"),
        AbiType::String => output.push_str("string"),
        AbiType::Bytes => output.push_str("bytes"),
        AbiType::FixedBytes(size) => {
            write!(output, "bytes{size}").expect("String formatting cannot fail");
        }
        AbiType::Uint(bits) => write!(output, "uint{bits}").expect("String formatting cannot fail"),
        AbiType::Int(bits) => write!(output, "int{bits}").expect("String formatting cannot fail"),
        AbiType::UFixed(bits, decimals) => {
            write!(output, "ufixed{bits}x{decimals}").expect("String formatting cannot fail");
        }
        AbiType::Fixed(bits, decimals) => {
            write!(output, "fixed{bits}x{decimals}").expect("String formatting cannot fail");
        }
        AbiType::Function => output.push_str("function"),
        AbiType::Array(element) => {
            canonical_type(element, output)?;
            output.push_str("[]");
        }
        AbiType::FixedArray(element, length) => {
            canonical_type(element, output)?;
            write!(output, "[{length}]").expect("String formatting cannot fail");
        }
        AbiType::Tuple(fields) => {
            output.push('(');
            for (index, field) in fields.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                canonical_type(&field.ty, output)?;
            }
            output.push(')');
        }
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte == b'$' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte == b'$' || byte.is_ascii_alphanumeric())
}
