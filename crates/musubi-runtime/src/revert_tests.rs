//! Strict public custom-error descriptor validation.

use musubi_core::{AbiField, AbiType, ErrorDescriptor};

use crate::{CodecError, decode_revert};

fn descriptor(name: &str, signature: &str, selector: [u8; 4]) -> ErrorDescriptor {
    ErrorDescriptor {
        name: name.into(),
        signature: signature.into(),
        selector,
        inputs: vec![AbiField {
            name: "value".into(),
            ty: AbiType::String,
            internal_type: None,
        }],
    }
}

#[test]
fn custom_errors_reject_forged_reserved_and_duplicate_descriptors() {
    let valid = descriptor("Failure", "Failure(string)", [0xfd, 0x5d, 0xe2, 0x76]);
    let mut forged_signature = valid.clone();
    forged_signature.signature = "Other(string)".into();
    let mut forged_selector = valid.clone();
    forged_selector.selector = [0; 4];
    let reserved = descriptor("Error", "Error(string)", [0x08, 0xc3, 0x79, 0xa0]);

    for errors in [
        vec![forged_signature],
        vec![forged_selector],
        vec![reserved],
        vec![valid.clone(), valid],
    ] {
        assert_eq!(decode_revert(&[], &errors), Err(CodecError::Selector));
    }
}
