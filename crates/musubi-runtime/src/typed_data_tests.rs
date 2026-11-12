//! Solidity-linked and adversarial EIP-712 engine vectors.

use musubi_core::{
    AbiField, AbiType, AbiValue, Address, Bytes, FixedBytes, TypedDataField, TypedDataStruct,
    TypedDataType, U256, keccak256,
};

use crate::{
    TypedDataError, TypedDataGraph, encode_fields, typed_data_digest, typed_data_struct_hash,
    typed_data_type_hash, typed_data_type_string,
};

fn field(name: &str, ty: TypedDataType) -> TypedDataField {
    TypedDataField {
        name: name.into(),
        ty,
    }
}

fn safe_types() -> Vec<TypedDataStruct> {
    vec![
        TypedDataStruct {
            name: "EIP712Domain".into(),
            fields: vec![
                field("chainId", TypedDataType::Uint(256)),
                field("verifyingContract", TypedDataType::Address),
            ],
        },
        TypedDataStruct {
            name: "SafeTx".into(),
            fields: vec![
                field("to", TypedDataType::Address),
                field("value", TypedDataType::Uint(256)),
                field("data", TypedDataType::Bytes),
                field("operation", TypedDataType::Uint(8)),
                field("safeTxGas", TypedDataType::Uint(256)),
                field("baseGas", TypedDataType::Uint(256)),
                field("gasPrice", TypedDataType::Uint(256)),
                field("gasToken", TypedDataType::Address),
                field("refundReceiver", TypedDataType::Address),
                field("nonce", TypedDataType::Uint(256)),
            ],
        },
    ]
}

fn uint(value: u128) -> AbiValue {
    AbiValue::Uint(U256::from_u128(value).unwrap().to_be_bytes())
}

fn hash(value: &str) -> FixedBytes<32> {
    FixedBytes::from_slice(&hex::decode(value.trim_start_matches("0x")).unwrap()).unwrap()
}

#[test]
fn safe_hashes_match_independent_foundry_golden_vector() {
    let types = safe_types();
    let domain = AbiValue::Tuple(vec![
        uint(11_155_111),
        AbiValue::Address(
            "0x1234567890abcdef1234567890abcdef12345678"
                .parse()
                .unwrap(),
        ),
    ]);
    let transaction = AbiValue::Tuple(vec![
        AbiValue::Address(
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .parse()
                .unwrap(),
        ),
        uint(0),
        AbiValue::Bytes(Bytes::from_hex("0xdeadbeef").unwrap()),
        uint(0),
        uint(0),
        uint(0),
        uint(0),
        AbiValue::Address(Address::ZERO),
        AbiValue::Address(Address::ZERO),
        uint(42),
    ]);
    assert_eq!(
        typed_data_type_string(&types, "SafeTx").unwrap(),
        "SafeTx(address to,uint256 value,bytes data,uint8 operation,uint256 safeTxGas,uint256 baseGas,uint256 gasPrice,address gasToken,address refundReceiver,uint256 nonce)"
    );
    assert_eq!(
        typed_data_struct_hash(&types, "EIP712Domain", &domain).unwrap(),
        hash("0x5f9246b4dfb61b8c1856ebf915f25401aaa441edd3e618dfa685abb7bc11c9ff")
    );
    assert_eq!(
        typed_data_struct_hash(&types, "SafeTx", &transaction).unwrap(),
        hash("0xded7dce09e5387a4fa1b08a0e258e2aedd1ea2ecd52c874693d7a522ee05b152")
    );
    assert_eq!(
        typed_data_digest(&types, "EIP712Domain", &domain, "SafeTx", &transaction).unwrap(),
        hash("0x1f0cb86e5d37170b03e06c8ea3dc8faa9d3c5d13a2480c6db788ac6276e34de3")
    );
}

#[test]
fn dependency_order_and_arrays_match_external_solidity_golden() {
    let types = vec![
        TypedDataStruct {
            name: "Person".into(),
            fields: vec![
                field("name", TypedDataType::String),
                field("wallet", TypedDataType::Address),
            ],
        },
        TypedDataStruct {
            name: "Group".into(),
            fields: vec![field(
                "members",
                TypedDataType::Array(Box::new(TypedDataType::Struct("Person".into()))),
            )],
        },
    ];
    assert_eq!(
        typed_data_type_string(&types, "Group").unwrap(),
        "Group(Person[] members)Person(string name,address wallet)"
    );
    assert_eq!(
        typed_data_type_hash(&types, "Group").unwrap(),
        hash("0xf3adab05a0b3b07c9011fd977cdcc754e58ead1e1e726bc7485fd838981f0368")
    );
    let person = |name: &str, byte| {
        AbiValue::Tuple(vec![
            AbiValue::String(name.into()),
            AbiValue::Address(Address::new([byte; 20])),
        ])
    };
    let first = AbiValue::Tuple(vec![AbiValue::Array(vec![
        person("Alice", 1),
        person("Bob", 2),
    ])]);
    let second = AbiValue::Tuple(vec![AbiValue::Array(vec![
        person("Bob", 2),
        person("Alice", 1),
    ])]);
    // This fixed result was obtained independently from the consumer-owned Solidity formula:
    // keccak256(abi.encode(groupTypeHash, keccak256(abi.encodePacked(personHashes)))).
    assert_eq!(
        typed_data_struct_hash(&types, "Group", &first).unwrap(),
        hash("0x6ceef4da3b8605bf55265a94cccf738064cb934b396231e09588e696d49bb357")
    );
    assert_ne!(
        typed_data_struct_hash(&types, "Group", &first).unwrap(),
        typed_data_struct_hash(&types, "Group", &second).unwrap()
    );
}

#[test]
fn abi_encoding_and_eip712_hashing_are_separate_authorities() {
    let types = vec![TypedDataStruct {
        name: "Payload".into(),
        fields: vec![field("data", TypedDataType::Bytes)],
    }];
    let bytes = Bytes::from_hex("0xdeadbeef").unwrap();
    let typed_hash = typed_data_struct_hash(
        &types,
        "Payload",
        &AbiValue::Tuple(vec![AbiValue::Bytes(bytes.clone())]),
    )
    .unwrap();
    let abi = encode_fields(
        &[AbiField {
            name: "data".into(),
            ty: AbiType::Bytes,
            internal_type: None,
        }],
        &[AbiValue::Bytes(bytes)],
    )
    .unwrap();
    assert_ne!(typed_hash, keccak256(abi.as_ref()));
}

#[test]
fn graph_cycles_and_noncanonical_narrow_values_fail_closed() {
    let cycle = vec![TypedDataStruct {
        name: "Loop".into(),
        fields: vec![field("next", TypedDataType::Struct("Loop".into()))],
    }];
    assert_eq!(
        typed_data_type_hash(&cycle, "Loop"),
        Err(TypedDataError::InvalidGraph)
    );

    let narrow = vec![TypedDataStruct {
        name: "Narrow".into(),
        fields: vec![field("value", TypedDataType::Uint(8))],
    }];
    let mut out_of_range = [0; 32];
    out_of_range[30] = 1;
    assert_eq!(
        typed_data_struct_hash(
            &narrow,
            "Narrow",
            &AbiValue::Tuple(vec![AbiValue::Uint(out_of_range)])
        ),
        Err(TypedDataError::TypeMismatch)
    );

    let signed = vec![TypedDataStruct {
        name: "Signed".into(),
        fields: vec![field("value", TypedDataType::Int(8))],
    }];
    let mut invalid_sign_extension = [0; 32];
    invalid_sign_extension[31] = 0x80;
    assert_eq!(
        typed_data_struct_hash(
            &signed,
            "Signed",
            &AbiValue::Tuple(vec![AbiValue::Int(invalid_sign_extension)])
        ),
        Err(TypedDataError::TypeMismatch)
    );

    let fixed_array = vec![TypedDataStruct {
        name: "Pair".into(),
        fields: vec![field(
            "values",
            TypedDataType::FixedArray(Box::new(TypedDataType::Uint(8)), 2),
        )],
    }];
    assert_eq!(
        typed_data_struct_hash(
            &fixed_array,
            "Pair",
            &AbiValue::Tuple(vec![AbiValue::Array(vec![uint(1)])])
        ),
        Err(TypedDataError::TypeMismatch)
    );
}

#[test]
fn shared_dependency_dag_validation_is_memoized() {
    let mut types = vec![TypedDataStruct {
        name: "Node0".into(),
        fields: vec![field("value", TypedDataType::Uint(256))],
    }];
    types.push(TypedDataStruct {
        name: "Node1".into(),
        fields: vec![field("left", TypedDataType::Struct("Node0".into()))],
    });
    for index in 2..40 {
        types.push(TypedDataStruct {
            name: format!("Node{index}"),
            fields: vec![
                field("left", TypedDataType::Struct(format!("Node{}", index - 1))),
                field("right", TypedDataType::Struct(format!("Node{}", index - 2))),
            ],
        });
    }
    assert!(typed_data_type_hash(&types, "Node39").is_ok());
}

#[test]
fn repeated_nested_values_reuse_prepared_graph_and_type_hashes() {
    let repeated = 4_096;
    let types = vec![
        TypedDataStruct {
            name: "Leaf".into(),
            fields: vec![field("value", TypedDataType::Uint(256))],
        },
        TypedDataStruct {
            name: "Root".into(),
            fields: (0..repeated)
                .map(|index| {
                    field(
                        &format!("leaf{index}"),
                        TypedDataType::Struct("Leaf".into()),
                    )
                })
                .collect(),
        },
    ];
    let value = AbiValue::Tuple(
        (0..repeated)
            .map(|_| AbiValue::Tuple(vec![uint(1)]))
            .collect(),
    );
    let mut graph = TypedDataGraph::new(&types).unwrap();
    assert!(graph.struct_hash("Root", &value).is_ok());
    assert_eq!(graph.memoized_type_count(), 2);
}

#[test]
fn value_hashing_accepts_the_full_sixty_four_struct_edges() {
    let mut types = vec![TypedDataStruct {
        name: "Node0".into(),
        fields: vec![field("value", TypedDataType::Uint(256))],
    }];
    let mut value = AbiValue::Tuple(vec![uint(1)]);
    for index in 1..=64 {
        types.push(TypedDataStruct {
            name: format!("Node{index}"),
            fields: vec![field(
                "child",
                TypedDataType::Struct(format!("Node{}", index - 1)),
            )],
        });
        value = AbiValue::Tuple(vec![value]);
    }
    assert!(typed_data_struct_hash(&types, "Node64", &value).is_ok());
    types.push(TypedDataStruct {
        name: "Node65".into(),
        fields: vec![field("child", TypedDataType::Struct("Node64".into()))],
    });
    assert!(TypedDataGraph::new(&types).is_err());
}

#[test]
fn large_dynamic_array_hashing_streams_element_words() {
    let types = vec![TypedDataStruct {
        name: "Batch".into(),
        fields: vec![field(
            "values",
            TypedDataType::Array(Box::new(TypedDataType::Uint(256))),
        )],
    }];
    let value = AbiValue::Tuple(vec![AbiValue::Array(
        (0..65_536).map(|_| uint(1)).collect(),
    )]);
    assert!(typed_data_struct_hash(&types, "Batch", &value).is_ok());
}

#[test]
fn type_string_stops_during_construction_at_the_caller_budget() {
    let types = vec![TypedDataStruct {
        name: "Wide".into(),
        fields: (0..65_536)
            .map(|index| field(&format!("value{index}"), TypedDataType::Uint(256)))
            .collect(),
    }];
    let graph = TypedDataGraph::new(&types).unwrap();
    assert_eq!(
        graph.type_string_with_limit("Wide", 8),
        Err(TypedDataError::ResourceLimit)
    );
}

#[test]
fn graph_rejects_cumulative_fixed_array_layout_overflow() {
    let types = vec![
        TypedDataStruct {
            name: "Leaf".into(),
            fields: vec![field(
                "values",
                TypedDataType::FixedArray(Box::new(TypedDataType::Uint(256)), 257),
            )],
        },
        TypedDataStruct {
            name: "Root".into(),
            fields: vec![field(
                "leaves",
                TypedDataType::FixedArray(Box::new(TypedDataType::Struct("Leaf".into())), 256),
            )],
        },
    ];
    assert_eq!(
        TypedDataGraph::new(&types).unwrap_err(),
        TypedDataError::InvalidGraph
    );
}
