//! Deterministic generated item names, including overload suffixes.

use musubi_core::{ContractAbi, ErrorDescriptor, EventDescriptor, FunctionDescriptor, keccak256};

use crate::names::{rust_pascal, rust_snake};

pub(crate) fn function_type_name(abi: &ContractAbi, function: &FunctionDescriptor) -> String {
    let mut name = rust_pascal(&function.name);
    if abi
        .functions
        .iter()
        .filter(|item| item.name == function.name)
        .count()
        > 1
    {
        name.push_str(&hex::encode(function.selector));
    }
    name
}

pub(crate) fn function_method_name(abi: &ContractAbi, function: &FunctionDescriptor) -> String {
    let mut name = rust_snake(&function.name);
    if abi
        .functions
        .iter()
        .filter(|item| item.name == function.name)
        .count()
        > 1
    {
        name.push('_');
        name.push_str(&hex::encode(function.selector));
    }
    name
}

pub(crate) fn event_type_name(abi: &ContractAbi, event: &EventDescriptor) -> String {
    let mut name = rust_pascal(&event.name);
    if abi
        .events
        .iter()
        .filter(|item| item.name == event.name)
        .count()
        > 1
    {
        let signature_hash = keccak256(event.signature.as_bytes());
        name.push_str(&hex::encode(&signature_hash.as_ref()[..4]));
    }
    name
}

pub(crate) fn error_type_name(abi: &ContractAbi, error: &ErrorDescriptor) -> String {
    let mut name = rust_pascal(&error.name);
    if abi
        .errors
        .iter()
        .filter(|item| item.name == error.name)
        .count()
        > 1
    {
        name.push_str(&hex::encode(error.selector));
    }
    name
}
