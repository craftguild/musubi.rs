//! Committed external ABI golden vectors and malformed-input checks.

use musubi_core::{AbiField, AbiType, AbiValue, Address, Bytes, Uint};

use crate::{CodecError, codec::encode_function, decode_fields, encode_fields};

fn field(name: &str, ty: AbiType) -> AbiField {
    AbiField {
        name: name.into(),
        ty,
        internal_type: None,
    }
}

#[test]
fn transfer_call_matches_external_golden_vector() {
    let fields = vec![
        field("to", AbiType::Address),
        field("amount", AbiType::Uint(256)),
    ];
    let address = Address::new([0x11; 20]);
    let amount = Uint::<256>::from_u128(1).unwrap();
    let encoded = encode_function(
        [0xa9, 0x05, 0x9c, 0xbb],
        &fields,
        &[
            AbiValue::Address(address),
            AbiValue::Uint(amount.to_be_bytes()),
        ],
    )
    .unwrap();
    let expected = concat!(
        "a9059cbb",
        "0000000000000000000000001111111111111111111111111111111111111111",
        "0000000000000000000000000000000000000000000000000000000000000001"
    );
    assert_eq!(hex::encode(encoded.as_ref()), expected);
}

#[test]
fn dynamic_sequence_matches_external_golden_vector() {
    let fields = vec![
        field("message", AbiType::String),
        field("values", AbiType::Array(Box::new(AbiType::Uint(256)))),
    ];
    let one = Uint::<256>::from_u128(1).unwrap().to_be_bytes();
    let two = Uint::<256>::from_u128(2).unwrap().to_be_bytes();
    let values = vec![
        AbiValue::String("hello".into()),
        AbiValue::Array(vec![AbiValue::Uint(one), AbiValue::Uint(two)]),
    ];
    let expected = concat!(
        "0000000000000000000000000000000000000000000000000000000000000040",
        "0000000000000000000000000000000000000000000000000000000000000080",
        "0000000000000000000000000000000000000000000000000000000000000005",
        "68656c6c6f000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000002",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000002"
    );
    let encoded = encode_fields(&fields, &values).unwrap();
    assert_eq!(hex::encode(encoded.as_ref()), expected);
    assert_eq!(decode_fields(&fields, encoded.as_ref()).unwrap(), values);
}

#[test]
fn zero_decimal_fixed_values_round_trip_as_scaled_integers() {
    let mut negative = [0xff; 32];
    negative[31] = 0xf9;
    let fields = [
        field("unsigned", AbiType::UFixed(8, 0)),
        field("signed", AbiType::Fixed(256, 0)),
    ];
    let values = [
        AbiValue::UFixed(Uint::<8>::from_u128(7).unwrap().to_be_bytes()),
        AbiValue::Fixed(negative),
    ];
    let encoded = encode_fields(&fields, &values).unwrap();
    assert_eq!(decode_fields(&fields, encoded.as_ref()).unwrap(), values);
}

#[test]
fn decode_rejects_trailing_and_noncanonical_data() {
    let fields = [field("value", AbiType::Bytes)];
    let encoded = encode_fields(&fields, &[AbiValue::Bytes(Bytes::new([1, 2, 3]))]).unwrap();
    let mut trailing = encoded.into_vec();
    trailing.extend_from_slice(&[0; 32]);
    assert_eq!(
        decode_fields(&fields, &trailing),
        Err(CodecError::Malformed)
    );
}

#[test]
fn codec_rejects_invalid_public_descriptor_before_indexing() {
    let fields = [field("bad", AbiType::FixedBytes(33))];
    assert_eq!(
        encode_fields(&fields, &[AbiValue::FixedBytes(Bytes::new([0; 33]))]),
        Err(CodecError::TypeMismatch)
    );
    let overflowing = [field(
        "bad",
        AbiType::FixedArray(
            Box::new(AbiType::FixedArray(
                Box::new(AbiType::Uint(256)),
                usize::MAX,
            )),
            2,
        ),
    )];
    assert_eq!(
        decode_fields(&overflowing, &[]),
        Err(CodecError::TypeMismatch)
    );

    let at_limit = [field(
        "value",
        AbiType::FixedArray(
            Box::new(AbiType::FixedArray(Box::new(AbiType::Uint(256)), 256)),
            256,
        ),
    )];
    assert_eq!(decode_fields(&at_limit, &[]), Err(CodecError::Malformed));
    let over_limit = [field(
        "value",
        AbiType::FixedArray(
            Box::new(AbiType::FixedArray(Box::new(AbiType::Uint(256)), 257)),
            256,
        ),
    )];
    assert_eq!(
        decode_fields(&over_limit, &[]),
        Err(CodecError::TypeMismatch)
    );
}
