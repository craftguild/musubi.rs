//! Prefix-bounded structural rejection and unchanged large canonical ABI acceptance.

use std::io::{self, Read};

use super::{parse, preflight};
use crate::{DiagnosticCode, HardLimits, semantic_abi_modules};

/// A read beyond the expected rejection prefix becomes a syntax/I/O failure instead of a limit
/// diagnostic. This proves the guard does not traverse or materialize the unbounded suffix.
struct PrefixReader<'a> {
    bytes: &'a [u8],
    position: usize,
    permitted: usize,
}

impl Read for PrefixReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.permitted {
            return Err(io::Error::other("read past resource boundary"));
        }
        let count = output
            .len()
            .min(self.permitted - self.position)
            .min(self.bytes.len() - self.position);
        output[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

fn rejects_before_tail(bytes: &[u8], permitted: usize, limits: HardLimits) {
    let mut reader = PrefixReader {
        bytes,
        position: 0,
        permitted,
    };
    let mut deserializer = serde_json::Deserializer::from_reader(&mut reader);
    assert_eq!(
        preflight(&mut deserializer, limits).unwrap_err().code,
        DiagnosticCode::LimitExceeded
    );
    assert!(reader.position <= permitted);
    assert!(reader.position < bytes.len() / 2);
}

#[test]
fn abi_json_millions_of_items_stop_before_the_first_over_budget_body() {
    let bytes = format!("[{}0]", "0,".repeat(3_999_999)).into_bytes();
    assert_eq!(bytes.len(), 8_000_001);
    let permitted = 1 + HardLimits::V1.abi_items * 2 + 1;
    rejects_before_tail(&bytes, permitted, HardLimits::V1);
    assert_eq!(
        semantic_abi_modules(&bytes).unwrap_err().diagnostic.code,
        DiagnosticCode::LimitExceeded
    );
}

#[test]
fn abi_json_millions_of_nested_components_stop_before_the_first_over_budget_body() {
    let prefix = r#"[{"type":"function","name":"f","inputs":[{"type":"tuple","components":["#;
    let bytes = format!("{prefix}{}0]}}]}}]", "0,".repeat(3_999_999)).into_bytes();
    // The containing tuple already consumes one component from the aggregate ABI budget.
    let permitted = prefix.len() + (HardLimits::V1.abi_components - 1) * 2 + 1;
    rejects_before_tail(&bytes, permitted, HardLimits::V1);
    assert_eq!(
        semantic_abi_modules(&bytes).unwrap_err().diagnostic.code,
        DiagnosticCode::LimitExceeded
    );
}

#[test]
fn abi_json_component_budget_is_shared_across_items_inputs_outputs_and_tuples() {
    let bytes = br#"[
      {"type":"function","name":"f","inputs":[{"type":"uint256"}],"outputs":[{"type":"uint256"}]},
      {"type":"event","name":"E","inputs":[{"type":"tuple","components":[{"type":"uint256"}]}]}
    ]"#;
    let mut limits = HardLimits::V1;
    limits.abi_items = 2;
    limits.abi_components = 4;
    let values = parse(bytes, limits).unwrap();
    crate::abi_parse::parse_abi(&values, limits).unwrap();
    limits.abi_components = 3;
    assert_eq!(
        parse(bytes, limits).unwrap_err().code,
        DiagnosticCode::LimitExceeded
    );
    limits.abi_components = 4;
    limits.abi_items = 1;
    assert_eq!(
        parse(bytes, limits).unwrap_err().code,
        DiagnosticCode::LimitExceeded
    );
}

#[test]
fn abi_json_depth_limits_stop_long_before_the_nested_suffix() {
    let prefix = r#"[{"type":"function","name":"f","inputs":["#;
    let tuple = r#"{"type":"tuple","components":["#;
    let bytes = format!(
        "{prefix}{}{{\"type\":\"uint256\"}}{}]}}]",
        tuple.repeat(10_000),
        "]}".repeat(10_000)
    );
    let mut limits = HardLimits::V1;
    limits.nesting = 3;
    rejects_before_tail(bytes.as_bytes(), prefix.len() + tuple.len() * 4 + 1, limits);
    // The existing JSON recursion ceiling is also a resource error, not a generic invalid ABI.
    rejects_before_tail(
        bytes.as_bytes(),
        prefix.len() + tuple.len() * 64 + 1,
        HardLimits::V1,
    );
    assert_eq!(
        semantic_abi_modules(bytes.as_bytes())
            .unwrap_err()
            .diagnostic
            .code,
        DiagnosticCode::LimitExceeded
    );
}

#[test]
fn abi_json_exact_nested_boundary_and_empty_component_arrays_are_accepted() {
    let bytes = br#"[{"type":"function","name":"f","inputs":[{"type":"tuple","components":[{"type":"uint256","components":[]}]}]}]"#;
    let mut limits = HardLimits::V1;
    limits.nesting = 1;
    let values = parse(bytes, limits).unwrap();
    crate::abi_parse::parse_abi(&values, limits).unwrap();
}

#[test]
fn abi_json_closed_keys_and_shapes_cannot_hide_unbudgeted_subtrees() {
    for bytes in [
        r#"[{"type":"function","unknown":[0,0,0]}]"#,
        r#"[{"type":"function","inputs":[],"inpu\u0074s":[]}]"#,
        r#"[{"type":"function","inputs":[{"type":"uint256","type":"uint256"}]}]"#,
        r#"[{"type":[0,0,0]}]"#,
        r#"[{"name":{"unexpected":[0,0,0]}}]"#,
        "[[0,0,0]]",
        r#"{"abi":[]}"#,
        "[] trailing",
    ] {
        assert_eq!(
            parse(bytes.as_bytes(), HardLimits::V1).unwrap_err().code,
            DiagnosticCode::InvalidInput
        );
    }
}

#[test]
fn abi_json_large_canonical_assets_keep_existing_item_component_and_byte_limits() {
    let limits = HardLimits::V1;
    let fields = (0..16)
        .map(|index| {
            serde_json::json!({
                "name": format!("field_{index}_{}", "x".repeat(64)), "type":"uint256"
            })
        })
        .collect::<Vec<_>>();
    let abi = (0..limits.abi_items)
        .map(|index| {
            serde_json::json!({
                "type":"event", "name":format!("E{index}"), "inputs":fields
            })
        })
        .collect::<Vec<_>>();
    // 4,096 items and 65,536 fields exactly, using valid <=256-byte identifiers. Pretty canonical
    // ABI assets may exceed the original 8 MiB input cap and must retain the output byte allowance.
    assert!(serde_json::to_vec(&abi).unwrap().len() <= limits.abi_bytes);
    let bytes = crate::canonical::canonical_abi(&abi).unwrap();
    assert!(bytes.len() > limits.abi_bytes);
    assert!(bytes.len() < limits.generated_bytes);
    let owners = semantic_abi_modules(&bytes).unwrap();
    assert_eq!(owners.len(), limits.abi_items);
}
