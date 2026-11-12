//! Closed raw-string and compiler-object bytecode syntax tests.

use crate::{DiagnosticCode, HardLimits, bytecode::parse_creation};

#[test]
fn fully_linked_string_requires_lowercase_hex_prefix() {
    let error = parse_creation(br#""6000""#, HardLimits::V1).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InvalidInput);
    assert!(parse_creation(br#""0X6000""#, HardLimits::V1).is_err());
    assert!(parse_creation(br#""0x6000""#, HardLimits::V1).is_ok());
}

#[test]
fn compiler_object_may_omit_the_hex_prefix() {
    assert!(parse_creation(br#"{"object":"6000"}"#, HardLimits::V1).is_ok());
}
