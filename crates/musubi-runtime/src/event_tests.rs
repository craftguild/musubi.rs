//! Event wire vectors and adversarial canonical-decoding checks.

use musubi_core::{
    AbiField, AbiType, AbiValue, Address, Bytes, EventDescriptor, EventField, Hash, Int, Log, Uint,
};

use crate::{CodecError, DecodedEventField, decode_event, encode_event};

fn event_field(name: &str, ty: AbiType, indexed: bool) -> EventField {
    EventField {
        field: AbiField {
            name: name.into(),
            ty,
            internal_type: None,
        },
        indexed,
    }
}

fn hash(value: &str) -> Hash {
    Hash::new(hex::decode(value).unwrap().try_into().unwrap())
}

fn log(topics: Vec<Hash>, data: Vec<u8>) -> Log {
    Log {
        address: Address::new([0xaa; 20]),
        topics,
        data: Bytes::new(data),
        block_hash: None,
        transaction_hash: None,
        log_index: None,
        removed: false,
    }
}

#[test]
fn transfer_matches_the_canonical_external_event_vector() {
    let topic = hash("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
    let descriptor = EventDescriptor {
        name: "Transfer".into(),
        signature: "Transfer(address,address,uint256)".into(),
        topic: Some(topic.into_array()),
        inputs: vec![
            event_field("from", AbiType::Address, true),
            event_field("to", AbiType::Address, true),
            event_field("value", AbiType::Uint(256), false),
        ],
        anonymous: false,
    };
    let values = vec![
        DecodedEventField::Value(AbiValue::Address(Address::new([0x11; 20]))),
        DecodedEventField::Value(AbiValue::Address(Address::new([0x22; 20]))),
        DecodedEventField::Value(AbiValue::Uint(
            Uint::<256>::from_u128(7).unwrap().to_be_bytes(),
        )),
    ];
    let encoded = encode_event(&descriptor, Address::new([0xaa; 20]), &values).unwrap();
    assert_eq!(encoded.topics[0], topic);
    assert_eq!(
        hex::encode(encoded.topics[1].as_ref()),
        "0000000000000000000000001111111111111111111111111111111111111111"
    );
    assert_eq!(
        hex::encode(encoded.topics[2].as_ref()),
        "0000000000000000000000002222222222222222222222222222222222222222"
    );
    assert_eq!(
        hex::encode(encoded.data.as_ref()),
        "0000000000000000000000000000000000000000000000000000000000000007"
    );
    assert_eq!(decode_event(&descriptor, &encoded).unwrap(), values);
    assert_eq!(
        (
            encoded.block_hash,
            encoded.transaction_hash,
            encoded.log_index,
            encoded.removed
        ),
        (None, None, None, false)
    );
}

#[test]
fn anonymous_signed_and_fixed_bytes_fields_match_word_encoding() {
    let descriptor = EventDescriptor {
        name: "Edge".into(),
        signature: "Edge(int24,bytes4,bytes4,int24)".into(),
        topic: None,
        inputs: vec![
            event_field("tick", AbiType::Int(24), true),
            event_field("indexed_tag", AbiType::FixedBytes(4), true),
            event_field("tag", AbiType::FixedBytes(4), false),
            event_field("delta", AbiType::Int(24), false),
        ],
        anonymous: true,
    };
    let minus_two = Int::<24>::from_i128(-2).unwrap().to_be_bytes();
    let values = vec![
        DecodedEventField::Value(AbiValue::Int(minus_two)),
        DecodedEventField::Value(AbiValue::FixedBytes(Bytes::new([0xde, 0xad, 0xbe, 0xef]))),
        DecodedEventField::Value(AbiValue::FixedBytes(Bytes::new([1, 2, 3, 4]))),
        DecodedEventField::Value(AbiValue::Int(minus_two)),
    ];
    let encoded = encode_event(&descriptor, Address::ZERO, &values).unwrap();
    assert_eq!(encoded.topics[0].as_ref(), &minus_two);
    assert_eq!(
        hex::encode(encoded.topics[1].as_ref()),
        "deadbeef00000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(
        hex::encode(encoded.data.as_ref()),
        concat!(
            "0102030400000000000000000000000000000000000000000000000000000000",
            "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe"
        )
    );
    assert_eq!(decode_event(&descriptor, &encoded).unwrap(), values);
}

#[test]
fn indexed_dynamic_values_remain_explicit_topic_hashes() {
    let descriptor = EventDescriptor {
        name: "Message".into(),
        signature: "Message(string)".into(),
        topic: None,
        inputs: vec![event_field("text", AbiType::String, true)],
        anonymous: true,
    };
    let values = [DecodedEventField::TopicHash(Hash::new([0x44; 32]))];
    let encoded = encode_event(&descriptor, Address::ZERO, &values).unwrap();
    assert_eq!(encoded.topics, vec![Hash::new([0x44; 32])]);
    assert_eq!(decode_event(&descriptor, &encoded).unwrap(), values);
    assert_eq!(
        encode_event(
            &descriptor,
            Address::ZERO,
            &[DecodedEventField::Value(AbiValue::String("secret".into()))]
        ),
        Err(CodecError::TypeMismatch)
    );
}

#[test]
fn decode_rejects_forged_topics_counts_and_descriptors() {
    let topic = hash("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
    let mut descriptor = EventDescriptor {
        name: "Transfer".into(),
        signature: "Transfer(address,address,uint256)".into(),
        topic: Some(topic.into_array()),
        inputs: vec![event_field("from", AbiType::Address, true)],
        anonymous: false,
    };
    assert_eq!(
        decode_event(&descriptor, &log(vec![topic, Hash::new([0; 32])], vec![])),
        Err(CodecError::Selector)
    );
    descriptor.inputs = vec![
        event_field("from", AbiType::Address, true),
        event_field("to", AbiType::Address, true),
        event_field("value", AbiType::Uint(256), false),
    ];
    assert_eq!(
        decode_event(&descriptor, &log(vec![Hash::new([0; 32]); 3], vec![])),
        Err(CodecError::Selector)
    );
    assert_eq!(
        decode_event(&descriptor, &log(vec![topic], vec![])),
        Err(CodecError::Arity)
    );
    descriptor.topic = None;
    assert_eq!(
        decode_event(&descriptor, &log(vec![topic, Hash::new([0; 32])], vec![])),
        Err(CodecError::Selector)
    );
    descriptor.anonymous = true;
    descriptor.inputs = (0..=4)
        .map(|index| event_field(&format!("v{index}"), AbiType::Uint(256), true))
        .collect();
    assert_eq!(
        encode_event(&descriptor, Address::ZERO, &[]),
        Err(CodecError::Arity)
    );
}

#[test]
fn indexed_static_topics_require_canonical_padding_and_sign_extension() {
    let cases = [
        (AbiType::Bool, "Edge(bool)", {
            let mut word = [0; 32];
            word[31] = 2;
            word
        }),
        (AbiType::Address, "Edge(address)", {
            let mut word = [0; 32];
            word[0] = 1;
            word
        }),
        (AbiType::Int(24), "Edge(int24)", {
            let mut word = [0; 32];
            word[29..].copy_from_slice(&[0xff, 0xff, 0xfe]);
            word
        }),
        (AbiType::FixedBytes(4), "Edge(bytes4)", {
            let mut word = [0; 32];
            word[..4].copy_from_slice(&[1, 2, 3, 4]);
            word[31] = 1;
            word
        }),
    ];
    for (ty, signature, word) in cases {
        let descriptor = EventDescriptor {
            name: "Edge".into(),
            signature: signature.into(),
            topic: None,
            inputs: vec![event_field("value", ty, true)],
            anonymous: true,
        };
        assert_eq!(
            decode_event(&descriptor, &log(vec![Hash::new(word)], vec![])),
            Err(CodecError::Malformed)
        );
    }
}

#[test]
fn nonindexed_data_rejects_trailing_and_malformed_words() {
    let descriptor = EventDescriptor {
        name: "Value".into(),
        signature: "Value(bool)".into(),
        topic: None,
        inputs: vec![event_field("value", AbiType::Bool, false)],
        anonymous: true,
    };
    let mut noncanonical = [0; 32];
    noncanonical[31] = 2;
    assert_eq!(
        decode_event(&descriptor, &log(vec![], noncanonical.to_vec())),
        Err(CodecError::Malformed)
    );
    assert_eq!(
        decode_event(&descriptor, &log(vec![], vec![0; 64])),
        Err(CodecError::Malformed)
    );
}
