//! Closed ABI JSON parsing into canonical, Alloy-independent descriptors.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{
    AbiField, ConstructorDescriptor, ContractAbi, ErrorDescriptor, EventDescriptor, EventField,
    FunctionDescriptor, StateMutability, keccak256,
};
use serde::Deserialize;

use crate::{
    Diagnostic, DiagnosticCode, HardLimits,
    abi_type::{RawField, canonical_type, fields},
    names::validate_abi_name,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawItem {
    #[serde(rename = "type")]
    kind: String,
    name: Option<String>,
    #[serde(default)]
    inputs: Vec<RawField>,
    #[serde(default)]
    outputs: Vec<RawField>,
    state_mutability: Option<String>,
    anonymous: Option<bool>,
}

pub(crate) fn parse_abi(
    values: &[serde_json::Value],
    limits: HardLimits,
) -> Result<ContractAbi, Diagnostic> {
    let mut abi = ContractAbi::default();
    let mut component_count = 0_usize;
    for value in values {
        let item: RawItem = serde_json::from_value(value.clone()).map_err(|_| invalid_item())?;
        match item.kind.as_str() {
            "constructor" => {
                if abi.constructor.is_some()
                    || item.name.is_some()
                    || !item.outputs.is_empty()
                    || item.anonymous.is_some()
                {
                    return Err(invalid_item());
                }
                let state_mutability = mutability(item.state_mutability.as_deref(), false)?;
                if matches!(
                    state_mutability,
                    StateMutability::View | StateMutability::Pure
                ) {
                    return Err(invalid_item());
                }
                abi.constructor = Some(ConstructorDescriptor {
                    inputs: fields(&item.inputs, limits, &mut component_count, false)?,
                    state_mutability,
                });
            }
            "function" => abi
                .functions
                .push(parse_function(&item, limits, &mut component_count)?),
            "event" => abi
                .events
                .push(parse_event(&item, limits, &mut component_count)?),
            "error" => abi
                .errors
                .push(parse_error(&item, limits, &mut component_count)?),
            "fallback" => set_special(&mut abi.fallback, &item, false)?,
            "receive" => set_special(&mut abi.receive, &item, true)?,
            _ => return Err(invalid_item()),
        }
        if component_count > limits.abi_components {
            return Err(Diagnostic::error(
                DiagnosticCode::LimitExceeded,
                "ABI component count exceeds the effective limit",
            ));
        }
    }
    abi.functions
        .sort_by(|left, right| left.signature.cmp(&right.signature));
    abi.events
        .sort_by(|left, right| left.signature.cmp(&right.signature));
    abi.errors
        .sort_by(|left, right| left.signature.cmp(&right.signature));
    validate_collisions(&abi)?;
    Ok(abi)
}

fn parse_function(
    item: &RawItem,
    limits: HardLimits,
    count: &mut usize,
) -> Result<FunctionDescriptor, Diagnostic> {
    let name = required_name(item, limits)?;
    if item.anonymous.is_some() {
        return Err(invalid_item());
    }
    let inputs = fields(&item.inputs, limits, count, false)?;
    let outputs = fields(&item.outputs, limits, count, false)?;
    let signature = signature(&name, &inputs);
    Ok(FunctionDescriptor {
        name,
        selector: selector(&signature),
        signature,
        inputs,
        outputs,
        state_mutability: mutability(item.state_mutability.as_deref(), false)?,
    })
}

fn parse_event(
    item: &RawItem,
    limits: HardLimits,
    count: &mut usize,
) -> Result<EventDescriptor, Diagnostic> {
    let name = required_name(item, limits)?;
    if !item.outputs.is_empty() || item.state_mutability.is_some() {
        return Err(invalid_item());
    }
    let parsed = fields(&item.inputs, limits, count, true)?;
    let inputs = parsed
        .into_iter()
        .zip(item.inputs.iter())
        .map(|(field, raw)| EventField {
            field,
            indexed: raw.indexed().unwrap_or(false),
        })
        .collect::<Vec<_>>();
    let signature = signature(
        &name,
        &inputs
            .iter()
            .map(|field| field.field.clone())
            .collect::<Vec<_>>(),
    );
    let anonymous = item.anonymous.unwrap_or(false);
    if inputs.iter().filter(|field| field.indexed).count() > if anonymous { 4 } else { 3 } {
        return Err(invalid_item());
    }
    Ok(EventDescriptor {
        name,
        topic: (!anonymous).then(|| keccak256(signature.as_bytes()).into_array()),
        signature,
        inputs,
        anonymous,
    })
}

fn parse_error(
    item: &RawItem,
    limits: HardLimits,
    count: &mut usize,
) -> Result<ErrorDescriptor, Diagnostic> {
    let name = required_name(item, limits)?;
    if !item.outputs.is_empty() || item.state_mutability.is_some() || item.anonymous.is_some() {
        return Err(invalid_item());
    }
    let inputs = fields(&item.inputs, limits, count, false)?;
    let signature = signature(&name, &inputs);
    Ok(ErrorDescriptor {
        name,
        selector: selector(&signature),
        signature,
        inputs,
    })
}

fn set_special(
    slot: &mut Option<StateMutability>,
    item: &RawItem,
    receive: bool,
) -> Result<(), Diagnostic> {
    if slot.is_some()
        || item.name.is_some()
        || !item.inputs.is_empty()
        || !item.outputs.is_empty()
        || item.anonymous.is_some()
    {
        return Err(invalid_item());
    }
    let value = mutability(item.state_mutability.as_deref(), receive)?;
    if (receive && value != StateMutability::Payable)
        || (!receive && matches!(value, StateMutability::View | StateMutability::Pure))
    {
        return Err(invalid_item());
    }
    *slot = Some(value);
    Ok(())
}

fn required_name(item: &RawItem, limits: HardLimits) -> Result<String, Diagnostic> {
    let name = item.name.clone().ok_or_else(invalid_item)?;
    if name.len() > limits.identifier_bytes {
        return Err(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            "ABI identifier exceeds the effective limit",
        ));
    }
    validate_abi_name(&name)?;
    Ok(name)
}

fn signature(name: &str, inputs: &[AbiField]) -> String {
    format!(
        "{name}({})",
        inputs
            .iter()
            .map(|field| canonical_type(&field.ty))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn selector(signature: &str) -> [u8; 4] {
    keccak256(signature.as_bytes()).as_array()[..4]
        .try_into()
        .unwrap_or([0; 4])
}

fn mutability(value: Option<&str>, receive: bool) -> Result<StateMutability, Diagnostic> {
    match value.unwrap_or(if receive { "payable" } else { "nonpayable" }) {
        "view" => Ok(StateMutability::View),
        "pure" => Ok(StateMutability::Pure),
        "nonpayable" => Ok(StateMutability::Nonpayable),
        "payable" => Ok(StateMutability::Payable),
        _ => Err(invalid_item()),
    }
}

fn validate_collisions(abi: &ContractAbi) -> Result<(), Diagnostic> {
    validate_unique(
        abi.functions
            .iter()
            .map(|item| (&item.signature, item.selector.to_vec())),
        "function selector",
    )?;
    validate_signatures(
        abi.events.iter().map(|item| &item.signature),
        "event signature",
    )?;
    validate_unique(
        abi.events
            .iter()
            .filter_map(|item| item.topic.map(|topic| (&item.signature, topic.to_vec()))),
        "event topic",
    )?;
    if abi.errors.iter().any(|item| {
        matches!(
            item.selector,
            [0x08, 0xc3, 0x79, 0xa0] | [0x4e, 0x48, 0x7b, 0x71]
        )
    }) {
        return Err(Diagnostic::error(
            DiagnosticCode::InconsistentArtifact,
            "custom error collides with a reserved Solidity error selector",
        ));
    }
    validate_unique(
        abi.errors
            .iter()
            .map(|item| (&item.signature, item.selector.to_vec())),
        "error selector",
    )
}

fn validate_signatures<'a>(
    signatures: impl Iterator<Item = &'a String>,
    kind: &str,
) -> Result<(), Diagnostic> {
    let mut unique = BTreeSet::new();
    for signature in signatures {
        if !unique.insert(signature) {
            return Err(Diagnostic::error(
                DiagnosticCode::InconsistentArtifact,
                format!("duplicate {kind}"),
            ));
        }
    }
    Ok(())
}

fn validate_unique<'a>(
    items: impl Iterator<Item = (&'a String, Vec<u8>)>,
    kind: &str,
) -> Result<(), Diagnostic> {
    let mut signatures = BTreeSet::new();
    let mut wires = BTreeMap::new();
    for (signature, wire) in items {
        if !signatures.insert(signature.clone()) || wires.insert(wire, signature).is_some() {
            return Err(Diagnostic::error(
                DiagnosticCode::InconsistentArtifact,
                format!("duplicate or colliding {kind}"),
            ));
        }
    }
    Ok(())
}

fn invalid_item() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "ABI item has invalid fields or state mutability",
    )
}
