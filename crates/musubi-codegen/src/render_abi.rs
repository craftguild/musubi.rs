//! Private generated ABI descriptor expression rendering.

use musubi_core::{AbiField, AbiType, ContractAbi, StateMutability};

use crate::{RenderError, render_source::BoundedSource};

pub(crate) fn function(
    source: &mut BoundedSource,
    function: &musubi_core::FunctionDescriptor,
) -> Result<(), RenderError> {
    source.push("mod abi {\nuse musubi_core::*;\n")?;
    source.formatted(format_args!(
        "pub(super) fn descriptor() -> FunctionDescriptor {{ FunctionDescriptor {{ name: {:?}.into(), signature: {:?}.into(), selector: {:?}, inputs: ",
        function.name, function.signature, function.selector
    ))?;
    fields_expr(source, &function.inputs)?;
    source.push(", outputs: ")?;
    fields_expr(source, &function.outputs)?;
    source.formatted(format_args!(
        ", state_mutability: {} }} }}\n}}\n",
        mutability_expr(function.state_mutability)
    ))
}

pub(crate) fn event(
    source: &mut BoundedSource,
    event: &musubi_core::EventDescriptor,
) -> Result<(), RenderError> {
    source.push(
        "pub(super) fn descriptor() -> musubi_core::EventDescriptor {\n    use musubi_core::*;\n",
    )?;
    source.formatted(format_args!(
        "EventDescriptor {{ name: {:?}.into(), signature: {:?}.into(), topic: ",
        event.name, event.signature
    ))?;
    match event.topic {
        Some(topic) => source.formatted(format_args!("Some({topic:?})"))?,
        None => source.push("None")?,
    }
    source.push(", inputs: vec![")?;
    for (index, field) in event.inputs.iter().enumerate() {
        separator(source, index)?;
        source.push("EventField { field: ")?;
        field_expr(source, &field.field)?;
        source.formatted(format_args!(", indexed: {} }}", field.indexed))?;
    }
    source.formatted(format_args!("], anonymous: {} }} }}\n", event.anonymous))
}

pub(crate) fn error(
    source: &mut BoundedSource,
    error: &musubi_core::ErrorDescriptor,
) -> Result<(), RenderError> {
    source.push(
        "pub(super) fn descriptor() -> musubi_core::ErrorDescriptor {\n    use musubi_core::*;\n",
    )?;
    source.formatted(format_args!(
        "ErrorDescriptor {{ name: {:?}.into(), signature: {:?}.into(), selector: {:?}, inputs: ",
        error.name, error.signature, error.selector
    ))?;
    fields_expr(source, &error.inputs)?;
    source.push(" } }\n")
}

pub(crate) fn constructor(
    source: &mut BoundedSource,
    abi: &ContractAbi,
) -> Result<(), RenderError> {
    source.push("fn constructor_inputs() -> Vec<musubi_core::AbiField> {\n")?;
    if abi
        .constructor
        .as_ref()
        .is_some_and(|item| !item.inputs.is_empty())
    {
        source.push("    use musubi_core::*;\n")?;
    }
    fields_expr(
        source,
        abi.constructor.as_ref().map_or(&[], |item| &item.inputs),
    )?;
    source.push(" }\n")
}

fn fields_expr(source: &mut BoundedSource, fields: &[AbiField]) -> Result<(), RenderError> {
    source.push("vec![")?;
    for (index, field) in fields.iter().enumerate() {
        separator(source, index)?;
        field_expr(source, field)?;
    }
    source.push("]")
}

fn field_expr(source: &mut BoundedSource, field: &AbiField) -> Result<(), RenderError> {
    source.formatted(format_args!(
        "AbiField {{ name: {:?}.into(), ty: ",
        field.name
    ))?;
    type_expr(source, &field.ty)?;
    source.push(", internal_type: ")?;
    match &field.internal_type {
        Some(value) => source.formatted(format_args!("Some({value:?}.into())"))?,
        None => source.push("None")?,
    }
    source.push(" }")
}

fn type_expr(source: &mut BoundedSource, ty: &AbiType) -> Result<(), RenderError> {
    match ty {
        AbiType::Address => source.push("AbiType::Address"),
        AbiType::Bool => source.push("AbiType::Bool"),
        AbiType::String => source.push("AbiType::String"),
        AbiType::Bytes => source.push("AbiType::Bytes"),
        AbiType::FixedBytes(size) => source.formatted(format_args!("AbiType::FixedBytes({size})")),
        AbiType::Uint(bits) => source.formatted(format_args!("AbiType::Uint({bits})")),
        AbiType::Int(bits) => source.formatted(format_args!("AbiType::Int({bits})")),
        AbiType::UFixed(bits, decimals) => {
            source.formatted(format_args!("AbiType::UFixed({bits}, {decimals})"))
        }
        AbiType::Fixed(bits, decimals) => {
            source.formatted(format_args!("AbiType::Fixed({bits}, {decimals})"))
        }
        AbiType::Function => source.push("AbiType::Function"),
        AbiType::Array(element) => {
            source.push("AbiType::Array(Box::new(")?;
            type_expr(source, element)?;
            source.push("))")
        }
        AbiType::FixedArray(element, length) => {
            source.push("AbiType::FixedArray(Box::new(")?;
            type_expr(source, element)?;
            source.formatted(format_args!("), {length})"))
        }
        AbiType::Tuple(fields) => {
            source.push("AbiType::Tuple(")?;
            fields_expr(source, fields)?;
            source.push(")")
        }
    }
}

fn separator(source: &mut BoundedSource, index: usize) -> Result<(), RenderError> {
    if index != 0 {
        source.push(", ")?;
    }
    Ok(())
}

fn mutability_expr(value: StateMutability) -> &'static str {
    match value {
        StateMutability::View => "StateMutability::View",
        StateMutability::Pure => "StateMutability::Pure",
        StateMutability::Nonpayable => "StateMutability::Nonpayable",
        StateMutability::Payable => "StateMutability::Payable",
    }
}
