//! Generated SDK differential checks against committed external tuple wire vectors.

use crate::output_test_support::{CargoValidation, generate};

use std::{fs, path::Path};

use serde::Deserialize;

use crate::{
    CARGO_PROCESS_LIMITS, HardLimits, ProcessRequest, abi_parse::parse_abi, build_ir, load_inputs,
    render, run_bounded_process,
};

#[derive(Deserialize)]
struct Golden {
    abi: Vec<serde_json::Value>,
    selector: String,
    calldata: String,
    return_data: String,
}

#[test]
fn generated_selector_calldata_and_return_decode_match_external_golden() {
    let golden: Golden =
        serde_json::from_str(include_str!("../tests/fixtures/owned-tuple-golden.json")).unwrap();
    let root = target_tempdir();
    write_generator_fixture(root.path(), &golden.abi);
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let ir = build_ir(inputs).unwrap();
    let abi = parse_abi(&golden.abi, HardLimits::V1).unwrap();
    assert_eq!(
        abi.functions[0].selector.as_slice(),
        decode_hex(&golden.selector)
    );

    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let sdk = root.path().join("sdk");
    generate(&tree, &sdk, CargoValidation::Required).unwrap();
    let consumer = root.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::create_dir(consumer.join("src")).unwrap();
    fs::write(consumer.join("Cargo.toml"), consumer_manifest()).unwrap();
    fs::write(
        consumer.join("src/lib.rs"),
        consumer_source(
            &decode_hex(&golden.calldata),
            &decode_hex(&golden.return_data),
        ),
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
        "consumer compile/test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_generator_fixture(root: &Path, abi: &[serde_json::Value]) {
    fs::write(root.join("abi.json"), serde_json::to_vec(abi).unwrap()).unwrap();
    fs::write(root.join("creation.json"), br#""0x60006000f3""#).unwrap();
    fs::write(
        root.join("musubi.toml"),
        concat!(
            "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n\n",
            "[generation]\noutput = \"sdk\"\n\n[package]\nname = \"owned-tuple-golden-sdk\"\n",
            "version = \"0.1.0\"\nedition = \"2024\"\n\n",
            "[[contracts]]\nkind = \"deployable\"\nid = \"planner\"\nabi = \"abi.json\"\n",
            "creation_bytecode = \"creation.json\"\n",
        ),
    )
    .unwrap();
}

fn consumer_manifest() -> &'static str {
    concat!(
        "[package]\nname = \"owned-tuple-golden-consumer\"\n",
        "version = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n",
        "[workspace]\n\n[dependencies]\n",
        "owned-tuple-golden-sdk = { path = \"../sdk\" }\n",
        "musubi-core = { path = \"../../../crates/musubi-core\" }\n",
        "musubi-runtime = { path = \"../../../crates/musubi-runtime\" }\n",
    )
}

fn consumer_source(calldata: &[u8], return_data: &[u8]) -> String {
    format!(
        r"//! Independent wire-vector consumer of the generated SDK.

#[cfg(test)]
mod tests {{
    use musubi_core::{{Address, Int}};
    use owned_tuple_golden_sdk::planner::{{
        DecideArgs, DecideReturn, Planner, Tuplea366b9f2ea8f039490e83d5a09fefd03,
        Tuplef8bbbf6550f06e7da4d52a9907c91296,
    }};

    #[test]
    fn generated_call_and_return_match_foundry_cast_vectors() {{
        let arguments = DecideArgs {{
            decision: Tuplef8bbbf6550f06e7da4d52a9907c91296 {{
                should_place: true,
                target_center_tick: Int::<24>::from_i128(-10).unwrap(),
                target_range_width: Int::<24>::from_i128(20).unwrap(),
            }},
        }};
        assert_eq!(arguments.clone().abi_encode().unwrap().as_ref(), &{calldata:?}[4..]);
        let request = Planner::decide(arguments)
            .at(Address::new([0x11; 20]))
            .call_request()
            .unwrap();
        assert_eq!(request.input.as_ref(), &{calldata:?});

        let decoded = DecideReturn::abi_decode(&{return_data:?}).unwrap();
        assert_eq!(decoded.clone().abi_encode().unwrap().as_ref(), &{return_data:?});
        assert_eq!(
            decoded.decision,
            Tuplea366b9f2ea8f039490e83d5a09fefd03 {{
                should_rebalance: false,
                target_center_tick: Int::<24>::from_i128(-11).unwrap(),
                target_range_width: Int::<24>::from_i128(21).unwrap(),
            }}
        );
    }}
}}
"
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

fn decode_hex(value: &str) -> Vec<u8> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).unwrap()
}
