//! Generated event encoding against compiler-derived ABI and cross-language topic evidence.

use crate::output_test_support::{CargoValidation, generate};

use std::{fs, path::Path};

use musubi_core::keccak256;

use crate::{
    CARGO_PROCESS_LIMITS, ProcessRequest, build_ir, load_inputs, render, run_bounded_process,
};

#[test]
fn anonymous_event_overloads_have_distinct_names_and_round_trip() {
    let root = target_tempdir();
    fs::write(
        root.path().join("abi.json"),
        r#"[
          {"type":"event","name":"Choice","anonymous":true,"inputs":[
            {"name":"id","type":"uint256","indexed":true},
            {"name":"label","type":"string","indexed":false}
          ]},
          {"type":"event","name":"Choice","anonymous":true,"inputs":[
            {"name":"id","type":"bytes32","indexed":true},
            {"name":"value","type":"uint256","indexed":false}
          ]}
        ]"#,
    )
    .unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        concat!(
            "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n\n",
            "[generation]\noutput = \"sdk\"\n\n[package]\nname = \"anonymous-event-sdk\"\n",
            "version = \"0.1.0\"\nedition = \"2024\"\n\n",
            "[[contracts]]\nid = \"events\"\n",
            "rust_module = \"events\"\nkind = \"interface\"\nabi = \"abi.json\"\n",
        ),
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
    let sdk = root.path().join("sdk");
    generate(&tree, &sdk, CargoValidation::Required).unwrap();

    let uint_name = overloaded_event_name("Choice(uint256,string)");
    let bytes_name = overloaded_event_name("Choice(bytes32,uint256)");
    assert_ne!(uint_name, bytes_name);
    assert!(!uint_name.ends_with("00000000"));
    assert!(!bytes_name.ends_with("00000000"));
    let generated = crate::output_test_support::rust_sources(&tree, "src/events/events");
    assert!(generated.contains(&format!("pub struct {uint_name}")));
    assert!(generated.contains(&format!("pub struct {bytes_name}")));

    let consumer = root.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::create_dir(consumer.join("src")).unwrap();
    fs::write(consumer.join("Cargo.toml"), anonymous_consumer_manifest()).unwrap();
    fs::write(
        consumer.join("src/lib.rs"),
        anonymous_consumer_source(&uint_name, &bytes_name),
    )
    .unwrap();
    let target = crate::output_test_support::cargo_target();
    let output = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments: &["test", "--quiet"],
            current_dir: &consumer,
            environment: &[("CARGO_TARGET_DIR", &target)],
        },
        CARGO_PROCESS_LIMITS,
    )
    .unwrap();
    assert!(
        output.status.success(),
        "anonymous event consumer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_rebalance_event_encodes_and_strictly_decodes_the_compiler_wire_shape() {
    let root = target_tempdir();
    fs::write(
        root.path().join("erc20.json"),
        include_bytes!("../../../external-acceptance/abi-corpus-v1/input/abi/erc20.json"),
    )
    .unwrap();
    fs::write(
        root.path().join("abi.json"),
        serde_json::to_vec(&serde_json::from_slice::<serde_json::Value>(include_bytes!(
            "../../../external-acceptance/event-rebalance-v1/solidity/out/RebalanceEventReference.t.sol/RebalanceEventReferenceTest.json"
        )).unwrap()["abi"]).unwrap(),
    )
    .unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        concat!(
            "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n\n",
            "[generation]\noutput = \"sdk\"\n\n[package]\nname = \"event-golden-sdk\"\n",
            "version = \"0.1.0\"\nedition = \"2024\"\n\n",
            "[[contracts]]\nid = \"sample_execution_events\"\n",
            "rust_module = \"sample_execution_events\"\nkind = \"interface\"\nabi = \"abi.json\"\n",
            "\n[[contracts]]\nid = \"erc20\"\nkind = \"interface\"\nabi = \"erc20.json\"\n",
        ),
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
    let sdk = root.path().join("sdk");
    generate(&tree, &sdk, CargoValidation::Required).unwrap();
    let generated =
        crate::output_test_support::rust_sources(&tree, "src/sample_execution_events/events");
    assert!(generated.contains("impl RebalanceExecuted"));
    assert!(generated.contains("pub fn encode_log(&self"));

    let consumer = root.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::create_dir(consumer.join("src")).unwrap();
    fs::write(consumer.join("Cargo.toml"), consumer_manifest()).unwrap();
    let vector: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../external-acceptance/event-rebalance-v1/vector.json"
    ))
    .unwrap();
    fs::write(consumer.join("src/lib.rs"), consumer_source(&vector)).unwrap();
    let target = crate::output_test_support::cargo_target();
    let output = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments: &["test", "--quiet"],
            current_dir: &consumer,
            environment: &[("CARGO_TARGET_DIR", &target)],
        },
        CARGO_PROCESS_LIMITS,
    )
    .unwrap();
    assert!(
        output.status.success(),
        "generated event consumer failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn consumer_manifest() -> &'static str {
    concat!(
        "[package]\nname = \"event-golden-consumer\"\n",
        "version = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n",
        "[workspace]\n\n[dependencies]\n",
        "event-golden-sdk = { path = \"../sdk\" }\n",
        "musubi-core = { path = \"../../../crates/musubi-core\" }\n",
        "hex = \"0.4\"\n",
    )
}

fn anonymous_consumer_manifest() -> &'static str {
    concat!(
        "[package]\nname = \"anonymous-event-consumer\"\n",
        "version = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n",
        "[workspace]\n\n[dependencies]\n",
        "anonymous-event-sdk = { path = \"../sdk\" }\n",
        "musubi-core = { path = \"../../../crates/musubi-core\" }\n",
    )
}

fn overloaded_event_name(signature: &str) -> String {
    let digest = keccak256(signature.as_bytes());
    format!("Choice{}", hex::encode(&digest.as_ref()[..4]))
}

fn anonymous_consumer_source(uint_name: &str, bytes_name: &str) -> String {
    r#"//! Anonymous overloaded event wire consumer.

#[cfg(test)]
mod tests {
    use anonymous_event_sdk::events::{BYTES_EVENT, UINT_EVENT};
    use musubi_core::{Address, FixedBytes, Uint};

    fn uint<const BITS: usize>(value: u128) -> Uint<BITS> {
        Uint::from_u128(value).unwrap()
    }

    #[test]
    fn both_anonymous_overloads_round_trip() {
        let uint_event = UINT_EVENT { id: uint::<256>(7), label: "seven".into() };
        let uint_log = uint_event.encode_log(Address::new([0x11; 20])).unwrap();
        assert_eq!(uint_log.topics.len(), 1);
        assert_eq!(UINT_EVENT::decode(&uint_log).unwrap(), uint_event);

        let bytes_event = BYTES_EVENT {
            id: FixedBytes::new([0x22; 32]),
            value: uint::<256>(8),
        };
        let bytes_log = bytes_event.encode_log(Address::new([0x33; 20])).unwrap();
        assert_eq!(bytes_log.topics.len(), 1);
        assert_eq!(BYTES_EVENT::decode(&bytes_log).unwrap(), bytes_event);
    }
}
"#
    .replace("UINT_EVENT", uint_name)
    .replace("BYTES_EVENT", bytes_name)
}

fn consumer_source(vector: &serde_json::Value) -> String {
    let topics = vector["event"]["topics"].as_array().unwrap();
    let mut source = r#"//! RebalanceExecuted wire consumer.

#[cfg(test)]
mod tests {
    use musubi_core::{Address, Bytes, FixedBytes, Hash, Int, Uint};
    use event_golden_sdk::sample_execution_events::RebalanceExecuted;

    fn uint<const BITS: usize>(value: u128) -> Uint<BITS> {
        Uint::from_u128(value).unwrap()
    }

    fn hash(value: &str) -> Hash {
        let bytes = hex::decode(value.strip_prefix("0x").unwrap()).unwrap();
        Hash::new(bytes.try_into().unwrap())
    }

    #[test]
    fn encode_log_matches_reviewed_topic_and_static_word_layout() {
        let event = RebalanceExecuted {
            policy_hash: FixedBytes::new([0x11; 32]),
            execution_nonce: uint::<256>(7),
            pool_config_hash: FixedBytes::new([0x22; 32]),
            plan_hash: FixedBytes::new([0x33; 32]),
            intent_hash: FixedBytes::new([0x44; 32]),
            old_position_identifier: uint::<256>(1),
            new_position_identifier: uint::<256>(2),
            tick_lower: Int::<24>::from_i128(-2).unwrap(),
            tick_upper: Int::<24>::from_i128(3).unwrap(),
            swap_amount0_in: uint::<256>(4),
            swap_amount1_in: uint::<256>(5),
            swap_amount0_out: uint::<256>(6),
            swap_amount1_out: uint::<256>(7),
            principal0_collected: uint::<256>(8),
            principal1_collected: uint::<256>(9),
            fee0_collected: uint::<256>(10),
            fee1_collected: uint::<256>(11),
            amount0_deposited: uint::<256>(12),
            amount1_deposited: uint::<256>(13),
            idle0_after: uint::<256>(14),
            idle1_after: uint::<256>(15),
            liquidity_added: uint::<128>(16),
        };
        let encoded = event.encode_log(Address::new([0xaa; 20])).unwrap();
        let expected_topics = vec![
            hash("TOPIC_0"),
            hash("TOPIC_1"),
            hash("TOPIC_2"),
            hash("TOPIC_3"),
        ];
        assert_eq!(encoded.topics, expected_topics);
        let expected_data = hex::decode("EVENT_DATA").unwrap();
        assert_eq!(encoded.data.as_ref(), expected_data);
        assert_eq!(RebalanceExecuted::decode(&encoded).unwrap(), event);

        let mut trailing = encoded.clone();
        let mut data = trailing.data.into_vec();
        data.extend_from_slice(&[0; 32]);
        trailing.data = Bytes::new(data);
        assert!(RebalanceExecuted::decode(&trailing).is_err());
    }

    #[test]
    fn public_facades_keep_type_identity() {
        use std::any::TypeId;
        assert_eq!(TypeId::of::<RebalanceExecuted>(),
            TypeId::of::<event_golden_sdk::sample_execution_events::types::RebalanceExecuted>());
        assert_eq!(TypeId::of::<event_golden_sdk::erc20::BalanceOfReturn>(),
            TypeId::of::<event_golden_sdk::erc20::types::BalanceOfReturn>());
    }

    #[test]
    fn standard_call_and_transaction_builders_preserve_wire_bytes() {
        use event_golden_sdk::erc20::{BalanceOfArgs, BalanceOfReturn, Erc20, TransferArgs};
        let args = TransferArgs { to: Address::ZERO, amount: uint::<256>(42) };
        let body = args.clone().abi_encode().unwrap();
        let transaction = Erc20::transfer(args).at(Address::ZERO).transaction_request().unwrap();
        assert_eq!(&transaction.input.as_ref()[4..], body.as_ref());
        let call = Erc20::balance_of(BalanceOfArgs { account: Address::ZERO })
            .at(Address::ZERO).call_request().unwrap();
        assert_eq!(&call.input.as_ref()[..4], &[0x70, 0xa0, 0x82, 0x31]);
        let encoded = BalanceOfReturn { value_0: uint::<256>(7) }.abi_encode().unwrap();
        assert_eq!(BalanceOfReturn::abi_decode(encoded.as_ref()).unwrap().value_0, uint::<256>(7));
    }

}
"#
    .to_owned();
    for (index, topic) in topics.iter().enumerate() {
        source = source.replace(&format!("TOPIC_{index}"), topic.as_str().unwrap());
    }
    source.replace(
        "EVENT_DATA",
        vector["event"]["data"]
            .as_str()
            .unwrap()
            .strip_prefix("0x")
            .unwrap(),
    )
}

fn target_tempdir() -> tempfile::TempDir {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    fs::create_dir_all(workspace.join("target")).unwrap();
    tempfile::tempdir_in(workspace.join("target")).unwrap()
}
