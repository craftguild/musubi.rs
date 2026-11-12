//! solc-compatible bytecode object parsing without compiler execution.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{BytecodeRange, Bytes, CreationBytecode, ImmutableReference, LinkReference};
use serde::Deserialize;

use crate::{Diagnostic, DiagnosticCode, HardLimits, names::validate_abi_name};

#[derive(Deserialize)]
#[serde(untagged)]
enum BytecodeJson {
    Hex(String),
    Object(BytecodeObject),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BytecodeObject {
    object: String,
    #[serde(default)]
    link_references: BTreeMap<String, BTreeMap<String, Vec<RangeJson>>>,
    #[serde(default)]
    immutable_references: BTreeMap<String, Vec<RangeJson>>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct RangeJson {
    start: usize,
    length: usize,
}

pub(crate) struct ParsedBytecode {
    pub bytes: Bytes,
    pub links: Vec<LinkReference>,
    pub immutables: Vec<ImmutableReference>,
}

pub(crate) fn parse_creation(
    raw: &[u8],
    limits: HardLimits,
) -> Result<CreationBytecode, Diagnostic> {
    let parsed = parse(raw, false, limits)?;
    CreationBytecode::new(parsed.bytes, parsed.links).map_err(|_| inconsistent_reference())
}

pub(crate) fn parse_runtime(
    raw: &[u8],
    limits: HardLimits,
) -> Result<musubi_core::RuntimeBytecode, Diagnostic> {
    let parsed = parse(raw, true, limits)?;
    musubi_core::RuntimeBytecode::new(parsed.bytes, parsed.links, parsed.immutables)
        .map_err(|_| inconsistent_reference())
}

fn parse(raw: &[u8], runtime: bool, limits: HardLimits) -> Result<ParsedBytecode, Diagnostic> {
    let document = crate::closed_json::parse(raw).map_err(|()| invalid_bytecode_json())?;
    let json: BytecodeJson =
        serde_json::from_value(document).map_err(|_| invalid_bytecode_json())?;
    let object = match json {
        BytecodeJson::Hex(object) => {
            if !object.starts_with("0x") {
                return Err(Diagnostic::error(
                    DiagnosticCode::InvalidInput,
                    "fully linked bytecode string must use a lowercase 0x prefix",
                ));
            }
            BytecodeObject {
                object,
                link_references: BTreeMap::new(),
                immutable_references: BTreeMap::new(),
            }
        }
        BytecodeJson::Object(object) => object,
    };
    if !runtime && !object.immutable_references.is_empty() {
        return Err(Diagnostic::error(
            DiagnosticCode::InvalidInput,
            "creation bytecode cannot declare immutableReferences",
        ));
    }
    let links = flatten_links(object.link_references);
    let immutables = flatten_immutables(object.immutable_references);
    if links.iter().any(|reference| {
        reference.source.len() > limits.identifier_bytes
            || reference.library.len() > limits.identifier_bytes
            || !musubi_core::is_canonical_source_path(&reference.source)
            || validate_abi_name(&reference.library).is_err()
            || reference.ranges.is_empty()
    }) || immutables.iter().any(|reference| {
        reference.id.is_empty()
            || reference.id.len() > limits.identifier_bytes
            || !reference.id.bytes().all(|byte| byte.is_ascii_digit())
            || reference.ranges.is_empty()
    }) {
        return Err(inconsistent_reference());
    }
    let bytes = decode_template(&object.object, &links)?;
    validate_nonoverlap(bytes.len(), &links, &immutables)?;
    Ok(ParsedBytecode {
        bytes,
        links,
        immutables,
    })
}

fn flatten_links(
    references: BTreeMap<String, BTreeMap<String, Vec<RangeJson>>>,
) -> Vec<LinkReference> {
    references
        .into_iter()
        .flat_map(|(source, libraries)| {
            libraries.into_iter().map(move |(library, ranges)| {
                let mut ranges = ranges.into_iter().map(to_range).collect::<Vec<_>>();
                ranges.sort_by_key(|range| (range.start, range.length));
                LinkReference {
                    source: source.clone(),
                    library,
                    ranges,
                }
            })
        })
        .collect()
}

fn flatten_immutables(references: BTreeMap<String, Vec<RangeJson>>) -> Vec<ImmutableReference> {
    references
        .into_iter()
        .map(|(id, ranges)| {
            let mut ranges = ranges.into_iter().map(to_range).collect::<Vec<_>>();
            ranges.sort_by_key(|range| (range.start, range.length));
            ImmutableReference { id, ranges }
        })
        .collect()
}

fn to_range(range: RangeJson) -> BytecodeRange {
    BytecodeRange {
        start: range.start,
        length: range.length,
    }
}

fn decode_template(object: &str, links: &[LinkReference]) -> Result<Bytes, Diagnostic> {
    let object = object.strip_prefix("0x").unwrap_or(object);
    if !object.len().is_multiple_of(2) {
        return Err(invalid_hex());
    }
    let mut normalized = object.as_bytes().to_vec();
    for link in links {
        for range in &link.ranges {
            let start = range
                .start
                .checked_mul(2)
                .ok_or_else(inconsistent_reference)?;
            let length = range
                .length
                .checked_mul(2)
                .ok_or_else(inconsistent_reference)?;
            let end = start
                .checked_add(length)
                .ok_or_else(inconsistent_reference)?;
            let slot = normalized
                .get_mut(start..end)
                .ok_or_else(inconsistent_reference)?;
            if slot.iter().any(|byte| !byte.is_ascii_hexdigit()) {
                if range.length != 20 || !is_placeholder(slot) {
                    return Err(inconsistent_reference());
                }
                slot.fill(b'0');
            }
        }
    }
    if normalized.iter().any(|byte| !byte.is_ascii_hexdigit()) {
        return Err(Diagnostic::error(
            DiagnosticCode::InconsistentArtifact,
            "unresolved bytecode placeholder has no matching link reference",
        ));
    }
    let decoded = hex::decode(normalized).map_err(|_| invalid_hex())?;
    Ok(Bytes::new(decoded))
}

fn is_placeholder(slot: &[u8]) -> bool {
    slot.len() == 40
        && slot.starts_with(b"__")
        && slot.ends_with(b"__")
        && slot[2..38]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'))
}

fn validate_nonoverlap(
    template_len: usize,
    links: &[LinkReference],
    immutables: &[ImmutableReference],
) -> Result<(), Diagnostic> {
    let mut occupied = BTreeSet::new();
    let ranges = links
        .iter()
        .flat_map(|reference| reference.ranges.iter())
        .chain(
            immutables
                .iter()
                .flat_map(|reference| reference.ranges.iter()),
        );
    for range in ranges {
        range
            .validate(template_len)
            .map_err(|_| inconsistent_reference())?;
        for index in range.start..range.start + range.length {
            if !occupied.insert(index) {
                return Err(Diagnostic::error(
                    DiagnosticCode::InconsistentArtifact,
                    "bytecode references overlap",
                ));
            }
        }
    }
    Ok(())
}

fn invalid_hex() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "bytecode object is not even-length hexadecimal",
    )
}

fn invalid_bytecode_json() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "bytecode must be a unique-key JSON hex string or closed compiler bytecode object",
    )
}

fn inconsistent_reference() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InconsistentArtifact,
        "bytecode reference does not match its template",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_declared_placeholder_to_zero_template() {
        let raw = br#"{"object":"0x73__$1234567890123456789012345678901234$__00","linkReferences":{"L.sol":{"L":[{"start":1,"length":20}]}}}"#;
        let bytecode = parse_creation(raw, HardLimits::V1).unwrap();
        assert_eq!(bytecode.template.len(), 22);
        assert_eq!(bytecode.links.len(), 1);
        assert!(
            bytecode.template.as_ref()[1..21]
                .iter()
                .all(|byte| *byte == 0)
        );
    }

    #[test]
    fn rejects_undeclared_placeholder() {
        let raw = br#""0x73__$1234567890123456789012345678901234$__00""#;
        assert_eq!(
            parse_creation(raw, HardLimits::V1).unwrap_err().code,
            DiagnosticCode::InconsistentArtifact
        );
    }

    #[test]
    fn canonicalizes_range_order_and_rejects_empty_inventories() {
        let unsorted = format!(
            r#"{{"object":"0x{}","linkReferences":{{"L.sol":{{"L":[{{"start":21,"length":20}},{{"start":0,"length":20}}]}}}}}}"#,
            "00".repeat(41)
        );
        let creation = parse_creation(unsorted.as_bytes(), HardLimits::V1).unwrap();
        assert_eq!(creation.links[0].ranges[0].start, 0);
        assert_eq!(creation.links[0].ranges[1].start, 21);

        let empty = br#"{"object":"0x00","linkReferences":{"L.sol":{"L":[]}}}"#;
        assert_eq!(
            parse_creation(empty, HardLimits::V1).unwrap_err().code,
            DiagnosticCode::InconsistentArtifact
        );
        let empty_name = br#"{"object":"0x000000000000000000000000000000000000000000","linkReferences":{"":{"L":[{"start":0,"length":20}]}}}"#;
        assert_eq!(
            parse_creation(empty_name, HardLimits::V1).unwrap_err().code,
            DiagnosticCode::InconsistentArtifact
        );
    }
}
