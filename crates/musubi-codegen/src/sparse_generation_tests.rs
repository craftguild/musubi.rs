//! Standalone Clippy coverage for mutually isolated sparse ABI surfaces.

use crate::output_test_support::{CargoValidation, generate};

use std::{fs, path::Path};

use crate::{
    CARGO_PROCESS_LIMITS, ProcessRequest, build_ir, load_inputs, render, run_bounded_process,
};

struct SparseCase {
    name: &'static str,
    marker: &'static str,
    abi: &'static str,
}

#[test]
fn isolated_sparse_abi_crates_pass_standalone_clippy() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let target = crate::output_test_support::cargo_target();
    for case in sparse_cases() {
        let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
        write_fixture(root.path(), &case);
        let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
        let tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
        let output = root.path().join("sdk");
        generate(&tree, &output, CargoValidation::Skip).unwrap();
        assert_conditional_surface(&output, &case, &tree);
        let private = crate::output_test_support::private_copy(&tree, root.path());
        if case.name == "empty" {
            write_no_custom_error_test(&private);
        }
        let result = run_bounded_process(
            ProcessRequest {
                program: "cargo",
                arguments: &["clippy", "--all-targets", "--", "-D", "warnings"],
                current_dir: &private,
                environment: &[("CARGO_TARGET_DIR", target.as_path())],
            },
            CARGO_PROCESS_LIMITS,
        )
        .unwrap();
        assert!(
            result.status.success(),
            "sparse ABI `{}` failed standalone Clippy: {}",
            case.name,
            String::from_utf8_lossy(&result.stderr)
        );
        if case.name == "empty" {
            run_no_custom_error_test(&private, target.as_path());
        }
        assert!(
            crate::output_test_support::check(&tree, &output)
                .unwrap()
                .is_clean()
        );
    }
}

fn sparse_cases() -> [SparseCase; 5] {
    [
        SparseCase {
            name: "empty",
            marker: "SparseEmpty",
            abi: "[]",
        },
        SparseCase {
            name: "fallback-receive",
            marker: "SparseFallbackReceive",
            abi: r#"[
              {"type":"fallback","stateMutability":"payable"},
              {"type":"receive","stateMutability":"payable"}
            ]"#,
        },
        SparseCase {
            name: "unit-function",
            marker: "SparseUnitFunction",
            abi: r#"[
              {"type":"function","name":"ping","stateMutability":"view","inputs":[],"outputs":[]}
            ]"#,
        },
        SparseCase {
            name: "event-only",
            marker: "SparseEventOnly",
            abi: r#"[
              {"type":"event","name":"SparseEventOnlyClient","anonymous":false,"inputs":[{"name":"IntoAbiValue","type":"address","indexed":false}]},
              {"type":"event","name":"FunctionCall","anonymous":false,"inputs":[]},
              {"type":"event","name":"FunctionTransaction","anonymous":false,"inputs":[]}
            ]"#,
        },
        SparseCase {
            name: "typed-function-without-errors",
            marker: "SparseTypedFunctionWithoutErrors",
            abi: r#"[
              {"type":"function","name":"read","stateMutability":"view","inputs":[{"name":"Hash","type":"uint256"}],"outputs":[{"name":"value","type":"uint256"}]}
            ]"#,
        },
    ]
}

fn assert_conditional_surface(output: &Path, case: &SparseCase, tree: &crate::GeneratedTree) {
    let module = format!("sparse_{}", case.name.replace('-', "_"));
    let source = fs::read_to_string(output.join("src").join(&module).join("mod.rs")).unwrap();
    let types = crate::output_test_support::rust_sources(tree, &format!("src/{module}"));
    let client = format!("{}Client", case.marker);
    match case.name {
        "empty" | "fallback-receive" => {
            assert!(!source.contains("pub use types::*;"));
            assert!(!source.contains(&client));
        }
        "unit-function" => {
            assert!(!source.contains("pub use types::*;"));
            assert!(source.contains(&format!("pub struct {client}<B>")));
            assert!(source.contains("pub fn connect<B>"));
        }
        "event-only" => {
            assert!(source.contains("pub use types::*;"));
            assert!(!source.contains(&client));
            assert!(!source.contains("use musubi_runtime::FunctionCall;"));
            assert!(types.contains("pub struct SparseEventOnlyClient"));
            assert!(types.contains("pub struct FunctionCall"));
            assert!(types.contains("pub struct FunctionTransaction"));
        }
        "typed-function-without-errors" => {
            assert!(source.contains("pub use types::*;"));
            assert!(source.contains(&format!("pub struct {client}<B>")));
            assert!(source.contains("DecodedRevert::Custom { .. }"));
            assert!(!source.contains("use musubi_runtime::FromAbiValue as _;"));
        }
        _ => unreachable!(),
    }
}

fn write_no_custom_error_test(output: &Path) {
    fs::create_dir(output.join("tests")).unwrap();
    fs::write(
        output.join("tests/no_custom_error.rs"),
        r#"use generated_empty_sdk::sparse_empty::{SparseEmpty, SparseEmptyError};

#[test]
fn unknown_bytes_are_preserved_and_malformed_standard_reverts_are_rejected() {
    let unknown = [0xde, 0xad, 0xbe, 0xef];
    let SparseEmptyError::Unknown(bytes) = SparseEmpty::decode_error(&unknown).unwrap() else {
        panic!("unknown selector was not retained as unknown bytes");
    };
    assert_eq!(bytes.as_ref(), unknown);
    assert!(SparseEmpty::decode_error(&[0x08, 0xc3, 0x79, 0xa0]).is_err());
}
"#,
    )
    .unwrap();
}

fn run_no_custom_error_test(output: &Path, target: &Path) {
    let result = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments: &["test", "--test", "no_custom_error"],
            current_dir: output,
            environment: &[("CARGO_TARGET_DIR", target)],
        },
        CARGO_PROCESS_LIMITS,
    )
    .unwrap();
    assert!(
        result.status.success(),
        "no-custom-error behavior test failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn write_fixture(root: &Path, case: &SparseCase) {
    let contract_id = case.name.replace('-', "_");
    fs::write(root.join("abi.json"), case.abi).unwrap();
    fs::write(root.join("creation.json"), br#""0x60006000f3""#).unwrap();
    fs::write(
        root.join("musubi.toml"),
        format!(
            concat!(
                "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n\n",
                "[generation]\noutput = \"sdk\"\n\n[package]\nname = \"generated-{}-sdk\"\n",
                "version = \"0.1.0\"\nedition = \"2024\"\n\n",
                "[[contracts]]\nkind = \"deployable\"\nid = \"sparse_{}\"\nabi = \"abi.json\"\n",
                "creation_bytecode = \"creation.json\"\n",
            ),
            case.name, contract_id
        ),
    )
    .unwrap();
}
