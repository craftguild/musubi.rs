//! Standard and custom Solidity revert decoding.

use std::collections::BTreeSet;

use musubi_core::{AbiValue, ErrorDescriptor, keccak256};

use crate::{CodecError, codec::decode_function_input, descriptor::canonical_signature};

const ERROR_SELECTOR: [u8; 4] = [0x08, 0xc3, 0x79, 0xa0];
const PANIC_SELECTOR: [u8; 4] = [0x4e, 0x48, 0x7b, 0x71];

/// Strictly decoded revert payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedRevert {
    /// Standard `Error(string)` reason.
    Error(String),
    /// Standard `Panic(uint256)` code as a full word.
    Panic([u8; 32]),
    /// One ABI-declared custom error.
    Custom {
        /// Canonical custom error signature.
        signature: String,
        /// Descriptor-ordered error arguments.
        values: Vec<AbiValue>,
    },
    /// Payload has no recognized selector.
    Unknown,
}

/// Decodes standard and supplied custom errors without guessing malformed payloads.
///
/// # Errors
///
/// Returns [`CodecError`] when a recognized selector has malformed or non-canonical arguments.
pub fn decode_revert(
    data: &[u8],
    custom_errors: &[ErrorDescriptor],
) -> Result<DecodedRevert, CodecError> {
    validate_custom_errors(custom_errors)?;
    let Some(selector) = data.get(..4) else {
        return Ok(DecodedRevert::Unknown);
    };
    if selector == ERROR_SELECTOR {
        let fields = [musubi_core::AbiField {
            name: "message".into(),
            ty: musubi_core::AbiType::String,
            internal_type: None,
        }];
        let values = decode_function_input(ERROR_SELECTOR, &fields, data)?;
        let [AbiValue::String(message)] = values.as_slice() else {
            return Err(CodecError::TypeMismatch);
        };
        return Ok(DecodedRevert::Error(message.clone()));
    }
    if selector == PANIC_SELECTOR {
        let fields = [musubi_core::AbiField {
            name: "code".into(),
            ty: musubi_core::AbiType::Uint(256),
            internal_type: None,
        }];
        let values = decode_function_input(PANIC_SELECTOR, &fields, data)?;
        let [AbiValue::Uint(code)] = values.as_slice() else {
            return Err(CodecError::TypeMismatch);
        };
        return Ok(DecodedRevert::Panic(*code));
    }
    if let Some(error) = custom_errors
        .iter()
        .find(|error| selector == error.selector)
    {
        let values = decode_function_input(error.selector, &error.inputs, data)?;
        return Ok(DecodedRevert::Custom {
            signature: error.signature.clone(),
            values,
        });
    }
    Ok(DecodedRevert::Unknown)
}

fn validate_custom_errors(custom_errors: &[ErrorDescriptor]) -> Result<(), CodecError> {
    let mut selectors = BTreeSet::new();
    for error in custom_errors {
        let signature = canonical_signature(&error.name, &error.inputs)?;
        let selector: [u8; 4] = keccak256(signature.as_bytes()).as_ref()[..4]
            .try_into()
            .map_err(|_| CodecError::Selector)?;
        if error.signature != signature
            || error.selector != selector
            || matches!(selector, ERROR_SELECTOR | PANIC_SELECTOR)
            || !selectors.insert(selector)
        {
            return Err(CodecError::Selector);
        }
    }
    Ok(())
}
