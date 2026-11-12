//! Standalone compilation and conformance when SDK modules shadow the standard library.

use std::{fs, path::Path};

use crate::{
    CARGO_PROCESS_LIMITS, ProcessRequest, build_ir, load_inputs,
    output_test_support::{CargoValidation, generate},
    render, run_bounded_process,
};

#[test]
fn std_contract_modules_pass_standalone_check_and_clippy() {
    check_modules(&[
        (
            "derived-contract",
            "[[contracts]]\nkind = \"interface\"\nid = \"Std\"\nabi = \"abi.json\"\n",
        ),
        (
            "explicit-contract",
            "[[contracts]]\nkind = \"interface\"\nid = \"Api\"\nrust_module = \"std\"\nabi = \"abi.json\"\n",
        ),
    ]);
}

#[test]
fn std_typed_data_module_passes_standalone_check_and_clippy() {
    check_modules(&[(
        "typed-data",
        concat!(
            "[[typed_data]]\nid = \"group\"\nrust_module = \"std\"\n",
            "authority = \"closed-schema-v1\"\nschema = \"schema.json\"\nvectors = \"vectors.json\"\n",
        ),
    )]);
}

fn check_modules(cases: &[(&str, &str)]) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let target = crate::output_test_support::cargo_target();
    for (name, entry) in cases {
        let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
        write_fixture(root.path(), &workspace, name, entry);
        let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
        let tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
        let output = root.path().join("sdk");
        generate(&tree, &output, CargoValidation::Skip).unwrap();
        let source = fs::read_to_string(output.join("src/lib.rs")).unwrap();
        assert!(source.contains("pub mod std;"), "case {name}");
        musubi_testkit::inspect_generated_crate(&output.canonicalize().unwrap()).unwrap();
        let private = crate::output_test_support::private_copy(&tree, root.path());
        for arguments in [
            vec!["check", "--all-targets"],
            vec!["clippy", "--all-targets", "--", "-D", "warnings"],
        ] {
            let result = run_bounded_process(
                ProcessRequest {
                    program: "cargo",
                    arguments: &arguments,
                    current_dir: &private,
                    environment: &[("CARGO_TARGET_DIR", target.as_path())],
                },
                CARGO_PROCESS_LIMITS,
            )
            .unwrap();
            assert!(
                result.status.success(),
                "std module case {name} failed {arguments:?}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        assert!(
            crate::output_test_support::check(&tree, &output)
                .unwrap()
                .is_clean()
        );
    }
}

fn write_fixture(root: &Path, workspace: &Path, name: &str, entry: &str) {
    fs::write(root.join("abi.json"), b"[]").unwrap();
    let typed_fixture = workspace.join("external-acceptance/typed-data-array-v1");
    for asset in ["schema.json", "vectors.json"] {
        fs::copy(typed_fixture.join(asset), root.join(asset)).unwrap();
    }
    fs::write(
        root.join("musubi.toml"),
        format!(
            concat!(
                "schema_version = \"musubi/config-v4\"\n",
                "[runtime]\nsource = \"path\"\nroot = \"../..\"\n",
                "[generation]\noutput = \"sdk\"\n",
                "[package]\nname = \"generated-std-{name}-sdk\"\n",
                "version = \"0.1.0\"\nedition = \"2024\"\n{entry}",
            ),
            name = name,
            entry = entry,
        ),
    )
    .unwrap();
}
