//! Bounded semantic validation for contract descriptors in untrusted snapshots.

use std::collections::BTreeSet;

use musubi_core::{AbiField, AbiType, ContractAbi, FixedBytes, StateMutability, keccak256};

use crate::{
    ContractSnapshot, EnrichmentKind, HardLimits, SnapshotError,
    ast_internal::{internal_type_occurrences, internal_wrappers_match, split_internal, wire_base},
    name_validation::validate_generated_names,
    names::{rust_pascal, validate_abi_name},
    owned_identity::OwnedTypeRegistry,
};

pub(crate) fn validate(
    contract: &ContractSnapshot,
    limits: HardLimits,
) -> Result<(), SnapshotError> {
    validate_abi(&contract.abi, limits)?;
    validate_enrichments(contract, limits)?;
    validate_provenance(contract)?;
    if let Some(deployment) = &contract.deployment {
        if !nonzero(&deployment.creation_template_keccak256) {
            return Err(SnapshotError::InvalidJson);
        }
        crate::snapshot_bytecode_validation::validate(
            deployment.creation_template_bytes,
            &deployment.creation_links,
            &[],
            limits,
        )?;
        if let Some(runtime) = &deployment.runtime {
            if !nonzero(&runtime.template_keccak256) {
                return Err(SnapshotError::InvalidJson);
            }
            crate::snapshot_bytecode_validation::validate(
                runtime.template_bytes,
                &runtime.links,
                &runtime.immutables,
                limits,
            )?;
        }
    }
    Ok(())
}

fn validate_abi(abi: &ContractAbi, limits: HardLimits) -> Result<(), SnapshotError> {
    let items = usize::from(abi.constructor.is_some())
        .checked_add(abi.functions.len())
        .and_then(|count| count.checked_add(abi.events.len()))
        .and_then(|count| count.checked_add(abi.errors.len()))
        .and_then(|count| count.checked_add(usize::from(abi.fallback.is_some())))
        .and_then(|count| count.checked_add(usize::from(abi.receive.is_some())))
        .ok_or(SnapshotError::InvalidJson)?;
    if items > limits.abi_items
        || abi.constructor.as_ref().is_some_and(|constructor| {
            matches!(
                constructor.state_mutability,
                StateMutability::View | StateMutability::Pure
            )
        })
        || abi
            .fallback
            .is_some_and(|value| matches!(value, StateMutability::View | StateMutability::Pure))
        || abi
            .receive
            .is_some_and(|value| value != StateMutability::Payable)
        || !strict_order(abi.functions.iter().map(|item| item.signature.as_str()))
        || !strict_order(abi.events.iter().map(|item| item.signature.as_str()))
        || !strict_order(abi.errors.iter().map(|item| item.signature.as_str()))
    {
        return Err(SnapshotError::InvalidJson);
    }
    let mut components = 0_usize;
    if let Some(constructor) = &abi.constructor {
        validate_fields(&constructor.inputs, 0, &mut components, limits, 1)?;
    }
    let mut function_wires = BTreeSet::new();
    for function in &abi.functions {
        validate_name(&function.name, limits)?;
        validate_fields(&function.inputs, 0, &mut components, limits, 1)?;
        validate_fields(&function.outputs, 0, &mut components, limits, 1)?;
        let signature = signature(&function.name, &function.inputs);
        let selector = selector(&signature);
        if function.signature != signature
            || function.selector != selector
            || !function_wires.insert(selector)
        {
            return Err(SnapshotError::InvalidJson);
        }
    }
    let mut event_wires = BTreeSet::new();
    for event in &abi.events {
        validate_name(&event.name, limits)?;
        for field in &event.inputs {
            validate_fields(
                std::slice::from_ref(&field.field),
                0,
                &mut components,
                limits,
                1,
            )?;
        }
        let fields = event
            .inputs
            .iter()
            .map(|field| field.field.clone())
            .collect::<Vec<_>>();
        let signature = signature(&event.name, &fields);
        let expected = (!event.anonymous).then(|| keccak256(signature.as_bytes()).into_array());
        let indexed = event.inputs.iter().filter(|field| field.indexed).count();
        if event.signature != signature
            || event.topic != expected
            || indexed > if event.anonymous { 4 } else { 3 }
            || event.topic.is_some_and(|topic| !event_wires.insert(topic))
        {
            return Err(SnapshotError::InvalidJson);
        }
    }
    let mut error_wires = BTreeSet::from([[0x08, 0xc3, 0x79, 0xa0], [0x4e, 0x48, 0x7b, 0x71]]);
    for error in &abi.errors {
        validate_name(&error.name, limits)?;
        validate_fields(&error.inputs, 0, &mut components, limits, 1)?;
        let signature = signature(&error.name, &error.inputs);
        let selector = selector(&signature);
        if error.signature != signature
            || error.selector != selector
            || !error_wires.insert(selector)
        {
            return Err(SnapshotError::InvalidJson);
        }
    }
    Ok(())
}

fn validate_fields(
    fields: &[AbiField],
    depth: usize,
    components: &mut usize,
    limits: HardLimits,
    fixed_elements: usize,
) -> Result<(), SnapshotError> {
    for field in fields {
        if field.name.len() > limits.identifier_bytes
            || (!field.name.is_empty() && validate_abi_name(&field.name).is_err())
        {
            return Err(SnapshotError::InvalidJson);
        }
        *components = components
            .checked_add(1)
            .ok_or(SnapshotError::InvalidJson)?;
        if *components > limits.abi_components {
            return Err(SnapshotError::InvalidJson);
        }
        validate_type(&field.ty, depth, components, limits, fixed_elements)?;
    }
    Ok(())
}

fn validate_type(
    ty: &AbiType,
    depth: usize,
    components: &mut usize,
    limits: HardLimits,
    fixed_elements: usize,
) -> Result<(), SnapshotError> {
    if depth > limits.nesting {
        return Err(SnapshotError::InvalidJson);
    }
    let nested_fixed_elements = match ty {
        AbiType::FixedArray(_, length) if *length != 0 => fixed_elements
            .checked_mul(*length)
            .ok_or(SnapshotError::InvalidJson)?,
        AbiType::FixedArray(_, _) => return Err(SnapshotError::InvalidJson),
        _ => fixed_elements,
    };
    if nested_fixed_elements > limits.abi_fixed_array_elements {
        return Err(SnapshotError::InvalidJson);
    }
    match ty {
        AbiType::FixedBytes(size) if !(1..=32).contains(size) => Err(SnapshotError::InvalidJson),
        AbiType::Uint(bits) | AbiType::Int(bits) if !(8..=256).contains(bits) || bits % 8 != 0 => {
            Err(SnapshotError::InvalidJson)
        }
        AbiType::UFixed(bits, decimals) | AbiType::Fixed(bits, decimals)
            if !(8..=256).contains(bits) || bits % 8 != 0 || *decimals > 80 =>
        {
            Err(SnapshotError::InvalidJson)
        }
        AbiType::Array(element) => {
            validate_type(element, nested(depth)?, components, limits, fixed_elements)
        }
        AbiType::FixedArray(element, _) => validate_type(
            element,
            nested(depth)?,
            components,
            limits,
            nested_fixed_elements,
        ),
        AbiType::Tuple(fields) => {
            validate_fields(fields, nested(depth)?, components, limits, fixed_elements)
        }
        _ => Ok(()),
    }
}

fn validate_enrichments(
    contract: &ContractSnapshot,
    limits: HardLimits,
) -> Result<(), SnapshotError> {
    if !strict_order(
        contract
            .enrichments
            .iter()
            .map(|item| item.internal_type.as_str()),
    ) {
        return Err(SnapshotError::InvalidJson);
    }
    let occurrences = internal_type_occurrences(&contract.abi, limits.nesting);
    for enrichment in &contract.enrichments {
        let (prefix, identity) = split_internal(
            &enrichment.internal_type,
            limits.identifier_bytes,
            limits.nesting,
        )
        .ok_or(SnapshotError::InvalidJson)?;
        if rust_pascal(
            identity
                .rsplit('.')
                .next()
                .ok_or(SnapshotError::InvalidJson)?,
        ) != enrichment.rust_name
        {
            return Err(SnapshotError::InvalidJson);
        }
        let fields = occurrences
            .get(&enrichment.internal_type)
            .ok_or(SnapshotError::InvalidJson)?;
        let wire = fields
            .first()
            .map(|field| wire_base(&field.ty))
            .ok_or(SnapshotError::InvalidJson)?;
        if fields
            .iter()
            .any(|field| !internal_wrappers_match(field, limits.nesting))
            || fields.iter().any(|field| wire_base(&field.ty) != wire)
            || !enrichment_matches(prefix, wire, &enrichment.kind, limits)
        {
            return Err(SnapshotError::InvalidJson);
        }
    }
    let owned = OwnedTypeRegistry::build(&contract.abi, &contract.enrichments, limits)
        .map_err(|_| SnapshotError::InvalidJson)?;
    validate_generated_names(
        &contract.id,
        contract.kind,
        &contract.abi,
        &contract.enrichments,
        &owned,
    )
    .map_err(|_| SnapshotError::InvalidJson)?;
    if owned.enrichment_count() != contract.enrichments.len() {
        return Err(SnapshotError::InvalidJson);
    }
    Ok(())
}

fn enrichment_matches(
    prefix: &str,
    wire: &AbiType,
    kind: &EnrichmentKind,
    limits: HardLimits,
) -> bool {
    match (prefix, wire, kind) {
        (
            "enum",
            AbiType::Uint(8),
            EnrichmentKind::Enum {
                variants,
                underlying: AbiType::Uint(8),
            },
        ) => {
            !variants.is_empty()
                && variants.len() <= 256
                && variants.iter().all(|name| {
                    name.len() <= limits.identifier_bytes && validate_abi_name(name).is_ok()
                })
        }
        ("type", wire, EnrichmentKind::UserDefinedValueType { underlying }) => wire == underlying,
        ("struct", AbiType::Tuple(wire), EnrichmentKind::Struct { fields }) => wire == fields,
        _ => false,
    }
}

fn validate_provenance(contract: &ContractSnapshot) -> Result<(), SnapshotError> {
    let values = [
        Some(contract.provenance.canonical_abi_sha256),
        Some(contract.provenance.raw_abi_sha256),
        contract.provenance.ast_sha256,
        contract.provenance.metadata_sha256,
        contract.provenance.storage_layout_sha256,
    ];
    if values.into_iter().flatten().all(|value| nonzero(&value)) {
        Ok(())
    } else {
        Err(SnapshotError::InvalidJson)
    }
}

fn signature(name: &str, fields: &[AbiField]) -> String {
    format!(
        "{name}({})",
        fields
            .iter()
            .map(|field| crate::abi_type::canonical_type(&field.ty))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn selector(signature: &str) -> [u8; 4] {
    keccak256(signature.as_bytes()).as_ref()[..4]
        .try_into()
        .unwrap_or([0; 4])
}

fn validate_name(name: &str, limits: HardLimits) -> Result<(), SnapshotError> {
    if name.len() > limits.identifier_bytes || validate_abi_name(name).is_err() {
        Err(SnapshotError::InvalidJson)
    } else {
        Ok(())
    }
}

fn nested(depth: usize) -> Result<usize, SnapshotError> {
    depth.checked_add(1).ok_or(SnapshotError::InvalidJson)
}

fn strict_order<T: Ord>(values: impl Iterator<Item = T>) -> bool {
    let values = values.collect::<Vec<_>>();
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn nonzero(value: &FixedBytes<32>) -> bool {
    value.as_ref().iter().any(|byte| *byte != 0)
}
