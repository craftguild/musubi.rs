//! Mandatory Solidity-linked typed-data vector parsing and recomputation.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{
    AbiValue, Address, Bytes, Hash, Sha256Digest, TypedDataRoot, TypedDataStruct, TypedDataType,
    is_canonical_artifact_target, is_canonical_compiler_version, is_canonical_source_path, sha256,
};
use serde::Deserialize;

use crate::{IrError, TypedDataExternalEvidence, TypedDataRootEvidence};

const VECTOR_VERSION: &str = "musubi/typed-data-vectors-v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorBundle {
    schema_version: String,
    evidence: EvidenceWire,
    vectors: Vec<VectorWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceWire {
    solidity_source_path: String,
    solidity_source_sha256: Sha256Digest,
    artifact_target: String,
    artifact_sha256: Sha256Digest,
    compiler_version: String,
    compiler_settings_sha256: Sha256Digest,
    vector_command: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorWire {
    root: String,
    domain_type: Option<String>,
    value: serde_json::Value,
    domain: Option<serde_json::Value>,
    type_string: String,
    type_hash: Hash,
    struct_hash: Hash,
    domain_type_string: Option<String>,
    domain_type_hash: Option<Hash>,
    domain_separator: Option<Hash>,
    digest: Option<Hash>,
}

pub(crate) fn verify(
    raw: &[u8],
    types: &[TypedDataStruct],
    roots: &[TypedDataRoot],
    root_evidence: &[TypedDataRootEvidence],
    maximum_vectors: usize,
) -> Result<TypedDataExternalEvidence, IrError> {
    let document = crate::closed_json::parse(raw).map_err(|()| {
        crate::typed_data_schema::failure(
            "typed-data vector bundle is not valid closed-schema JSON",
        )
    })?;
    crate::typed_data_hash_text::validate(&document)?;
    let bundle: VectorBundle = serde_json::from_value(document).map_err(|_| {
        crate::typed_data_schema::failure(
            "typed-data vector bundle is not valid closed-schema JSON",
        )
    })?;
    if bundle.schema_version != VECTOR_VERSION
        || bundle.vectors.is_empty()
        || bundle.vectors.len() > maximum_vectors
        || !evidence_valid(&bundle.evidence)
    {
        return Err(failure(
            "typed-data vector bundle identity or count is invalid",
        ));
    }
    let mut graph = musubi_runtime::TypedDataGraph::new(types)
        .map_err(|_| failure("typed-data graph is invalid"))?;
    let evidence_by_root = root_evidence
        .iter()
        .map(|item| {
            (
                (item.primary_type.as_str(), item.domain_type.as_deref()),
                item,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut covered = BTreeSet::new();
    for vector in &bundle.vectors {
        let key = (vector.root.as_str(), vector.domain_type.as_deref());
        let evidence = evidence_by_root
            .get(&key)
            .copied()
            .ok_or_else(|| failure("typed-data vector references an undeclared root"))?;
        if vector.type_string != evidence.type_string || vector.type_hash != evidence.type_hash {
            return Err(failure("typed-data vector type identity mismatch"));
        }
        let value = parse_struct_value(&graph, &vector.root, &vector.value)?;
        let struct_hash = graph
            .struct_hash(&vector.root, &value)
            .map_err(|_| failure("typed-data vector value is invalid"))?;
        if struct_hash != vector.struct_hash {
            return Err(failure("typed-data vector struct hash mismatch"));
        }
        verify_domain(&mut graph, evidence, vector, &value)?;
        covered.insert(key);
    }
    if covered.len() != roots.len() {
        return Err(failure(
            "typed-data vectors do not cover every declared root",
        ));
    }
    Ok(TypedDataExternalEvidence {
        solidity_source_sha256: bundle.evidence.solidity_source_sha256,
        artifact_target: bundle.evidence.artifact_target,
        artifact_sha256: bundle.evidence.artifact_sha256,
        compiler_version: bundle.evidence.compiler_version,
        compiler_settings_sha256: bundle.evidence.compiler_settings_sha256,
        vector_command_sha256: sha256(bundle.evidence.vector_command.as_bytes()),
    })
}

fn verify_domain(
    graph: &mut musubi_runtime::TypedDataGraph<'_>,
    evidence: &TypedDataRootEvidence,
    vector: &VectorWire,
    value: &AbiValue,
) -> Result<(), IrError> {
    match evidence.domain_type.as_deref() {
        Some(domain_type) => {
            if vector.domain_type_string != evidence.domain_type_string
                || vector.domain_type_hash != evidence.domain_type_hash
            {
                return Err(failure("typed-data vector domain type identity mismatch"));
            }
            let domain_raw = vector
                .domain
                .as_ref()
                .ok_or_else(|| failure("typed-data vector domain value is missing"))?;
            let domain = parse_struct_value(graph, domain_type, domain_raw)?;
            let separator = graph
                .struct_hash(domain_type, &domain)
                .map_err(|_| failure("typed-data domain value is invalid"))?;
            let digest = graph
                .digest(domain_type, &domain, &vector.root, value)
                .map_err(|_| failure("typed-data digest input is invalid"))?;
            if vector.domain_separator != Some(separator) || vector.digest != Some(digest) {
                return Err(failure(
                    "typed-data vector domain separator or digest mismatch",
                ));
            }
        }
        None if vector.domain.is_some()
            || vector.domain_type_string.is_some()
            || vector.domain_type_hash.is_some()
            || vector.domain_separator.is_some()
            || vector.digest.is_some() =>
        {
            return Err(failure("struct-hash-only vector contains domain evidence"));
        }
        None => {}
    }
    Ok(())
}

fn parse_struct_value(
    graph: &musubi_runtime::TypedDataGraph<'_>,
    name: &str,
    value: &serde_json::Value,
) -> Result<AbiValue, IrError> {
    let descriptor = graph
        .descriptor(name)
        .map_err(|_| failure("typed-data vector struct type is unresolved"))?;
    let object = value
        .as_object()
        .ok_or_else(|| failure("typed-data struct vector must be an object"))?;
    if object.len() != descriptor.fields.len()
        || descriptor
            .fields
            .iter()
            .any(|field| !object.contains_key(&field.name))
    {
        return Err(failure(
            "typed-data struct vector fields do not match the schema",
        ));
    }
    descriptor
        .fields
        .iter()
        .map(|field| parse_value(graph, &field.ty, &object[&field.name]))
        .collect::<Result<Vec<_>, _>>()
        .map(AbiValue::Tuple)
}

fn parse_value(
    graph: &musubi_runtime::TypedDataGraph<'_>,
    ty: &TypedDataType,
    value: &serde_json::Value,
) -> Result<AbiValue, IrError> {
    match ty {
        TypedDataType::Address => parse_address(value).map(AbiValue::Address),
        TypedDataType::Bool => value
            .as_bool()
            .ok_or_else(value_failure)
            .map(AbiValue::Bool),
        TypedDataType::String => value
            .as_str()
            .ok_or_else(value_failure)
            .map(|value| AbiValue::String(value.into())),
        TypedDataType::Bytes => parse_hex(value).map(AbiValue::Bytes),
        TypedDataType::FixedBytes(size) => {
            let bytes = parse_hex(value)?;
            if bytes.len() != *size {
                return Err(value_failure());
            }
            Ok(AbiValue::FixedBytes(bytes))
        }
        TypedDataType::Uint(bits) => {
            let word = decimal_word(value, false)?;
            if !unsigned_valid(*bits, &word) {
                return Err(value_failure());
            }
            Ok(AbiValue::Uint(word))
        }
        TypedDataType::Int(bits) => {
            let (word, negative) = signed_decimal_word(value)?;
            if !signed_valid(*bits, &word, negative) {
                return Err(value_failure());
            }
            Ok(AbiValue::Int(word))
        }
        TypedDataType::Struct(name) => parse_struct_value(graph, name, value),
        TypedDataType::Array(element) => parse_array(graph, element, None, value),
        TypedDataType::FixedArray(element, length) => {
            parse_array(graph, element, Some(*length), value)
        }
    }
}

fn parse_array(
    graph: &musubi_runtime::TypedDataGraph<'_>,
    element: &TypedDataType,
    length: Option<usize>,
    value: &serde_json::Value,
) -> Result<AbiValue, IrError> {
    let values = value.as_array().ok_or_else(value_failure)?;
    if length.is_some_and(|length| length != values.len()) {
        return Err(value_failure());
    }
    values
        .iter()
        .map(|value| parse_value(graph, element, value))
        .collect::<Result<Vec<_>, _>>()
        .map(AbiValue::Array)
}

fn parse_hex(value: &serde_json::Value) -> Result<Bytes, IrError> {
    let value = value.as_str().ok_or_else(value_failure)?;
    let raw = value.strip_prefix("0x").ok_or_else(value_failure)?;
    if raw.len() % 2 != 0
        || raw
            .bytes()
            .any(|byte| !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        return Err(value_failure());
    }
    Bytes::from_hex(raw).map_err(|_| value_failure())
}

fn parse_address(value: &serde_json::Value) -> Result<Address, IrError> {
    let value = value.as_str().ok_or_else(value_failure)?;
    let raw = value.strip_prefix("0x").ok_or_else(value_failure)?;
    if raw.len() != 40
        || raw
            .bytes()
            .any(|byte| !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        return Err(value_failure());
    }
    Address::from_hex(value).map_err(|_| value_failure())
}

fn decimal_word(value: &serde_json::Value, allow_negative: bool) -> Result<[u8; 32], IrError> {
    let value = value.as_str().ok_or_else(value_failure)?;
    let digits = if allow_negative {
        value.strip_prefix('-').unwrap_or(value)
    } else {
        value
    };
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(value_failure());
    }
    let mut word = [0_u8; 32];
    for digit in digits.bytes().map(|byte| byte - b'0') {
        let mut carry = u16::from(digit);
        for byte in word.iter_mut().rev() {
            let next = u16::from(*byte) * 10 + carry;
            *byte = next.to_le_bytes()[0];
            carry = next >> 8;
        }
        if carry != 0 {
            return Err(value_failure());
        }
    }
    Ok(word)
}

fn signed_decimal_word(value: &serde_json::Value) -> Result<([u8; 32], bool), IrError> {
    let text = value.as_str().ok_or_else(value_failure)?;
    let negative = text.starts_with('-');
    let mut word = decimal_word(value, true)?;
    if negative {
        if word.iter().all(|byte| *byte == 0) {
            return Err(value_failure());
        }
        for byte in &mut word {
            *byte = !*byte;
        }
        for byte in word.iter_mut().rev() {
            let (next, overflow) = byte.overflowing_add(1);
            *byte = next;
            if !overflow {
                break;
            }
        }
    }
    Ok((word, negative))
}

fn unsigned_valid(bits: usize, word: &[u8; 32]) -> bool {
    word[..(256 - bits) / 8].iter().all(|byte| *byte == 0)
}

fn signed_valid(bits: usize, word: &[u8; 32], negative: bool) -> bool {
    let prefix = (256 - bits) / 8;
    let sign = word[prefix] & 0x80 != 0;
    sign == negative
        && word[..prefix]
            .iter()
            .all(|byte| *byte == if negative { 0xff } else { 0 })
}

fn evidence_valid(evidence: &EvidenceWire) -> bool {
    is_canonical_source_path(&evidence.solidity_source_path)
        && is_canonical_artifact_target(&evidence.artifact_target)
        && is_canonical_compiler_version(&evidence.compiler_version)
        && !evidence.vector_command.is_empty()
}

fn value_failure() -> IrError {
    failure("typed-data vector value is malformed or outside its declared type")
}

fn failure(message: &str) -> IrError {
    crate::typed_data_schema::failure(message)
}
