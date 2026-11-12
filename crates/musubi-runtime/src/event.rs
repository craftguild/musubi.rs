//! Strict event topic and data decoding with explicit indexed hashes.

use musubi_core::{
    AbiField, AbiType, AbiValue, Address, Bytes, EventDescriptor, Hash, Log, keccak256,
};

use crate::{CodecError, decode_fields, descriptor::canonical_signature, encode_fields};

/// One decoded event field in original ABI order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedEventField {
    /// Directly decoded indexed-static or non-indexed value.
    Value(AbiValue),
    /// Topic hash for an indexed composite/dynamic value whose preimage is unavailable.
    TopicHash(Hash),
}

/// Encodes a deterministic fixture log from validated event fields.
///
/// Indexed dynamic/composite fields must be supplied as [`DecodedEventField::TopicHash`] because
/// their preimages are not recoverable from a log. Indexed static and non-indexed fields must be
/// supplied as [`DecodedEventField::Value`]. Mined-block metadata is intentionally left absent.
///
/// # Errors
///
/// Returns [`CodecError`] for a forged descriptor, wrong field variant or count, or a value that
/// does not canonically match its ABI type.
pub fn encode_event(
    descriptor: &EventDescriptor,
    address: Address,
    values: &[DecodedEventField],
) -> Result<Log, CodecError> {
    if values.len() != descriptor.inputs.len() {
        return Err(CodecError::Arity);
    }
    let expected_topic = validate_descriptor(descriptor)?;
    let mut topics = expected_topic
        .into_iter()
        .map(Hash::new)
        .collect::<Vec<_>>();
    let mut data_fields = Vec::new();
    let mut data_values = Vec::new();
    for (field, value) in descriptor.inputs.iter().zip(values) {
        match (field.indexed, indexed_is_hash(&field.field.ty), value) {
            (true, true, DecodedEventField::TopicHash(hash)) => topics.push(*hash),
            (true, false, DecodedEventField::Value(value)) => {
                let encoded = encode_fields(
                    std::slice::from_ref(&field.field),
                    std::slice::from_ref(value),
                )?;
                let word: [u8; 32] = encoded
                    .as_ref()
                    .try_into()
                    .map_err(|_| CodecError::TypeMismatch)?;
                topics.push(Hash::new(word));
            }
            (false, _, DecodedEventField::Value(value)) => {
                data_fields.push(field.field.clone());
                data_values.push(value.clone());
            }
            _ => return Err(CodecError::TypeMismatch),
        }
    }
    Ok(Log {
        address,
        topics,
        data: Bytes::new(encode_fields(&data_fields, &data_values)?.into_vec()),
        block_hash: None,
        transaction_hash: None,
        log_index: None,
        removed: false,
    })
}

/// Strictly decodes an Ethereum log according to one event descriptor.
///
/// # Errors
///
/// Returns [`CodecError`] if signature topic, topic count, indexed padding, or non-indexed data is
/// malformed or non-canonical.
pub fn decode_event(
    descriptor: &EventDescriptor,
    log: &Log,
) -> Result<Vec<DecodedEventField>, CodecError> {
    let expected_topic = validate_descriptor(descriptor)?;
    let signature_topics = usize::from(!descriptor.anonymous);
    let indexed_count = descriptor
        .inputs
        .iter()
        .filter(|field| field.indexed)
        .count();
    if log.topics.len() != signature_topics + indexed_count {
        return Err(CodecError::Arity);
    }
    if let Some(expected) = expected_topic
        && log.topics.first().map(Hash::as_array) != Some(&expected)
    {
        return Err(CodecError::Selector);
    }
    let non_indexed_fields = descriptor
        .inputs
        .iter()
        .filter(|field| !field.indexed)
        .map(|field| field.field.clone())
        .collect::<Vec<_>>();
    let mut non_indexed = decode_fields(&non_indexed_fields, log.data.as_ref())?.into_iter();
    let mut topics = log.topics.iter().skip(signature_topics);
    let mut values = Vec::with_capacity(descriptor.inputs.len());
    for event_field in &descriptor.inputs {
        if !event_field.indexed {
            values.push(DecodedEventField::Value(
                non_indexed.next().ok_or(CodecError::Arity)?,
            ));
            continue;
        }
        let topic = *topics.next().ok_or(CodecError::Arity)?;
        if indexed_is_hash(&event_field.field.ty) {
            values.push(DecodedEventField::TopicHash(topic));
        } else {
            let field = AbiField {
                name: event_field.field.name.clone(),
                ty: event_field.field.ty.clone(),
                internal_type: event_field.field.internal_type.clone(),
            };
            let decoded = decode_fields(&[field], topic.as_ref())?;
            values.push(DecodedEventField::Value(
                decoded.into_iter().next().ok_or(CodecError::Arity)?,
            ));
        }
    }
    Ok(values)
}

fn validate_descriptor(descriptor: &EventDescriptor) -> Result<Option<[u8; 32]>, CodecError> {
    if descriptor.signature
        != canonical_signature(
            &descriptor.name,
            &descriptor
                .inputs
                .iter()
                .map(|field| field.field.clone())
                .collect::<Vec<_>>(),
        )?
    {
        return Err(CodecError::Selector);
    }
    let expected_topic =
        (!descriptor.anonymous).then(|| keccak256(descriptor.signature.as_bytes()).into_array());
    if descriptor.topic != expected_topic {
        return Err(CodecError::Selector);
    }
    let maximum_indexed = if descriptor.anonymous { 4 } else { 3 };
    if descriptor
        .inputs
        .iter()
        .filter(|field| field.indexed)
        .count()
        > maximum_indexed
    {
        return Err(CodecError::Arity);
    }
    Ok(expected_topic)
}

pub(crate) fn indexed_is_hash(ty: &AbiType) -> bool {
    matches!(
        ty,
        AbiType::String
            | AbiType::Bytes
            | AbiType::Array(_)
            | AbiType::FixedArray(_, _)
            | AbiType::Tuple(_)
    )
}
