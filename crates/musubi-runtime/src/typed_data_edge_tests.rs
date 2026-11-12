//! Positive EIP-712 atomic and fixed-array edge vectors.

use musubi_core::{
    AbiValue, Bytes, FixedBytes, Int, TypedDataField, TypedDataStruct, TypedDataType, Uint,
};

use crate::{typed_data_struct_hash, typed_data_type_hash, typed_data_type_string};

fn hash(value: &str) -> FixedBytes<32> {
    FixedBytes::from_slice(&hex::decode(value.trim_start_matches("0x")).unwrap()).unwrap()
}

fn uint<const BITS: usize>(value: u128) -> AbiValue {
    AbiValue::Uint(Uint::<BITS>::from_u128(value).unwrap().to_be_bytes())
}

#[test]
fn signed_fixed_bytes_and_fixed_array_match_the_solidity_word_formula() {
    let types = [TypedDataStruct {
        name: "Edge".into(),
        fields: vec![
            TypedDataField {
                name: "tick".into(),
                ty: TypedDataType::Int(24),
            },
            TypedDataField {
                name: "tag".into(),
                ty: TypedDataType::FixedBytes(4),
            },
            TypedDataField {
                name: "weights".into(),
                ty: TypedDataType::FixedArray(Box::new(TypedDataType::Uint(16)), 2),
            },
        ],
    }];
    let value = AbiValue::Tuple(vec![
        AbiValue::Int(Int::<24>::from_i128(-8_388_608).unwrap().to_be_bytes()),
        AbiValue::FixedBytes(Bytes::from_hex("0xdeadbeef").unwrap()),
        AbiValue::Array(vec![uint::<16>(0), uint::<16>(65_535)]),
    ]);

    assert_eq!(
        typed_data_type_string(&types, "Edge").unwrap(),
        "Edge(int24 tick,bytes4 tag,uint16[2] weights)"
    );
    assert_eq!(
        typed_data_type_hash(&types, "Edge").unwrap(),
        hash("0x9725f5cb8d96d8246decae61272937776dcef90446bf67e742cd941d7d875985")
    );
    // Independent ABI-word construction, also cross-checked with viem:
    // keccak256(abi.encode(TYPEHASH, int24(-8388608), bytes4(0xdeadbeef),
    //     keccak256(abi.encode(uint16(0), uint16(65535))))).
    assert_eq!(
        typed_data_struct_hash(&types, "Edge", &value).unwrap(),
        hash("0xbcb950b91075be76a296e92895a1940cc3d691988906eac237a7c19e546ef545")
    );

    let reversed = AbiValue::Tuple(vec![
        AbiValue::Int(Int::<24>::from_i128(-8_388_608).unwrap().to_be_bytes()),
        AbiValue::FixedBytes(Bytes::from_hex("0xdeadbeef").unwrap()),
        AbiValue::Array(vec![uint::<16>(65_535), uint::<16>(0)]),
    ]);
    assert_ne!(
        typed_data_struct_hash(&types, "Edge", &value).unwrap(),
        typed_data_struct_hash(&types, "Edge", &reversed).unwrap()
    );
}
