//! Config-v3 artifact-kind validation and generated-surface tests.

#[cfg(unix)]
use crate::output_test_support::{CargoValidation, generate};

use std::{fs, path::Path};

use musubi_core::ArtifactKind;
use tempfile::TempDir;

#[cfg(unix)]
use crate::{CARGO_PROCESS_LIMITS, ProcessRequest, run_bounded_process};
use crate::{
    CompatibilityCategory, build_ir, build_snapshot, diff_snapshots, load_inputs, parse_snapshot,
    render, snapshot_json,
};

fn write_abi(root: &Path) {
    fs::write(
        root.join("abi.json"),
        br#"[
          {"type":"function","name":"owner","inputs":[],"outputs":[{"name":"","type":"address"}],"stateMutability":"view"},
          {"type":"event","name":"Ping","anonymous":false,"inputs":[]},
          {"type":"error","name":"Paused","inputs":[]}
        ]"#,
    )
    .unwrap();
}

fn manifest(contract: &str) -> String {
    format!(
        "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n\n[package]\nname = \"generated-interface-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[generation]\noutput = \"sdk\"\n\n[[contracts]]\nid = \"status\"\nabi = \"abi.json\"\n{contract}\n"
    )
}

#[cfg(unix)]
fn validate_mixed_standalone(root: &Path, tree: &crate::GeneratedTree) {
    let output = root.join("sdk");
    generate(tree, &output, CargoValidation::Required).unwrap();
    let private = crate::output_test_support::private_copy(tree, root);
    let target = crate::output_test_support::cargo_target();
    for arguments in [
        &["check", "--quiet"][..],
        &["test", "--quiet"][..],
        &["clippy", "--all-targets", "--", "-D", "warnings"][..],
        &["rustdoc", "--lib", "--", "-D", "warnings"][..],
    ] {
        let result = run_bounded_process(
            ProcessRequest {
                program: "cargo",
                arguments,
                current_dir: &private,
                environment: &[("CARGO_TARGET_DIR", target.as_path())],
            },
            CARGO_PROCESS_LIMITS,
        )
        .unwrap();
        assert!(
            result.status.success(),
            "mixed generated crate command {arguments:?} failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let inputs = load_inputs(&root.join("musubi.toml")).unwrap();
    crate::doctor(tree, &inputs.output).unwrap();
}

#[test]
fn config_v4_requires_kind_and_enforces_kind_fields() {
    let cases = [
        "",
        "kind = \"typo\"",
        "kind = \"deployable\"",
        "kind = \"interface\"\ncreation_bytecode = \"creation.json\"",
        "kind = \"interface\"\nruntime_bytecode = \"runtime.json\"",
        "kind = \"interface\"\nstorage_layout = \"storage.json\"",
    ];
    for contract in cases {
        let root = TempDir::new().unwrap();
        write_abi(root.path());
        fs::write(root.path().join("musubi.toml"), manifest(contract)).unwrap();
        assert!(
            load_inputs(&root.path().join("musubi.toml")).is_err(),
            "invalid config-v4 contract accepted: {contract}"
        );
    }
}

#[test]
fn interface_ir_and_release_have_no_deployment_identity() {
    let root = TempDir::new().unwrap();
    write_abi(root.path());
    fs::write(
        root.path().join("musubi.toml"),
        manifest("kind = \"interface\""),
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert_eq!(inputs.contracts[0].config.kind, ArtifactKind::Interface);
    assert!(inputs.contracts[0].creation.is_none());
    let ir = build_ir(inputs).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let files = tree
        .files()
        .map(|file| (file.path, file.contents))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(!files.contains_key(Path::new("assets/status/creation.bin")));
    assert!(!files.contains_key(Path::new("assets/status/runtime.bin")));
    let source = crate::output_test_support::rust_sources(&tree, "src/status");
    assert!(source.contains("pub fn interface_artifact()"));
    assert!(!source.contains("pub fn artifact()"));
    assert!(!source.contains("pub fn init_code("));
    assert!(!source.contains("pub fn deployment("));
    let release: serde_json::Value =
        serde_json::from_slice(&files[Path::new("musubi-release.json")]).unwrap();
    assert_eq!(release["schema_version"], "musubi/release-v4");
    assert_eq!(release["contracts"][0]["kind"], "interface");
    assert!(release["contracts"][0]["creation_template_keccak256"].is_null());
}

#[test]
fn interface_rejects_constructor_while_deployable_accepts_it() {
    for (kind, accepted) in [("interface", false), ("deployable", true)] {
        let root = TempDir::new().unwrap();
        fs::write(
            root.path().join("abi.json"),
            br#"[{"type":"constructor","inputs":[{"name":"owner","type":"address"}],"stateMutability":"nonpayable"}]"#,
        )
        .unwrap();
        let creation = if accepted {
            fs::write(root.path().join("creation.json"), br#""0x6000""#).unwrap();
            "\ncreation_bytecode = \"creation.json\""
        } else {
            ""
        };
        fs::write(
            root.path().join("musubi.toml"),
            manifest(&format!("kind = \"{kind}\"{creation}")),
        )
        .unwrap();
        let result = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap());
        assert_eq!(result.is_ok(), accepted, "unexpected {kind} result");
    }
}

#[test]
fn deployment_method_name_is_reserved_only_for_deployable_artifacts() {
    for (kind, accepted) in [("interface", true), ("deployable", false)] {
        let root = TempDir::new().unwrap();
        fs::write(
            root.path().join("abi.json"),
            br#"[{"type":"function","name":"deployment","inputs":[],"outputs":[],"stateMutability":"view"}]"#,
        )
        .unwrap();
        let creation = if kind == "deployable" {
            fs::write(root.path().join("creation.json"), br#""0x6000""#).unwrap();
            "\ncreation_bytecode = \"creation.json\""
        } else {
            ""
        };
        fs::write(
            root.path().join("musubi.toml"),
            manifest(&format!("kind = \"{kind}\"{creation}")),
        )
        .unwrap();
        let result = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap());
        assert_eq!(result.is_ok(), accepted, "unexpected {kind} result");
    }
}

#[test]
fn deployable_only_import_names_remain_available_to_interfaces() {
    for (kind, accepted) in [("interface", true), ("deployable", false)] {
        let root = TempDir::new().unwrap();
        fs::write(
            root.path().join("abi.json"),
            br#"[{"type":"function","name":"request","inputs":[],"outputs":[{"name":"value","type":"tuple","internalType":"struct Api.TransactionRequest","components":[{"name":"nonce","type":"uint256"}]}],"stateMutability":"view"}]"#,
        )
        .unwrap();
        fs::write(
            root.path().join("ast.json"),
            br#"{"nodeType":"SourceUnit","nodes":[{"nodeType":"StructDefinition","name":"TransactionRequest","canonicalName":"Api.TransactionRequest","members":[{"nodeType":"VariableDeclaration","name":"nonce","typeDescriptions":{"typeString":"uint256"}}]}]}"#,
        )
        .unwrap();
        let creation = if kind == "deployable" {
            fs::write(root.path().join("creation.json"), br#""0x6000""#).unwrap();
            "\ncreation_bytecode = \"creation.json\""
        } else {
            ""
        };
        fs::write(
            root.path().join("musubi.toml"),
            manifest(&format!("kind = \"{kind}\"\nast = \"ast.json\"{creation}")),
        )
        .unwrap();
        let result = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap());
        assert_eq!(result.is_ok(), accepted, "unexpected {kind} result");
    }
}

#[test]
fn mixed_contract_crate_emits_kind_specific_surfaces() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    write_abi(root.path());
    fs::write(root.path().join("creation.json"), br#""0x6000""#).unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[package]
name = "generated-mixed-sdk"
version = "0.1.0"
edition = "2024"

[generation]
output = "sdk"

[[contracts]]
id = "deployed"
kind = "deployable"
abi = "abi.json"
creation_bytecode = "creation.json"

[[contracts]]
id = "external"
kind = "interface"
abi = "abi.json"
"#,
    )
    .unwrap();
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let files = tree
        .files()
        .map(|file| (file.path, file.contents))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(files.contains_key(Path::new("assets/deployed/creation.bin")));
    assert!(!files.contains_key(Path::new("assets/external/creation.bin")));
    let deployed = crate::output_test_support::rust_sources(&tree, "src/deployed");
    let external = crate::output_test_support::rust_sources(&tree, "src/external");
    assert!(deployed.contains("pub fn deployment("));
    assert!(!external.contains("pub fn deployment("));
    let release: serde_json::Value =
        serde_json::from_slice(&files[Path::new("musubi-release.json")]).unwrap();
    assert_eq!(release["contracts"][0]["kind"], "deployable");
    assert_eq!(release["contracts"][1]["kind"], "interface");

    #[cfg(unix)]
    validate_mixed_standalone(root.path(), &tree);
}

#[test]
fn current_snapshot_round_trip_and_kind_change_are_explicit() {
    let root = TempDir::new().unwrap();
    write_abi(root.path());
    fs::write(root.path().join("creation.json"), br#""0x6000""#).unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        manifest("kind = \"deployable\"\ncreation_bytecode = \"creation.json\""),
    )
    .unwrap();
    let deployable = build_snapshot(
        &build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap(),
        &crate::test_generator::info(),
    );
    let candidate = parse_snapshot(&snapshot_json(&deployable).unwrap()).unwrap();
    assert_eq!(candidate, deployable);
    assert_eq!(
        candidate.contracts[0]
            .deployment
            .as_ref()
            .unwrap()
            .creation_template_bytes,
        2
    );

    let mut interface = candidate.clone();
    interface.contracts[0].kind = ArtifactKind::Interface;
    interface.contracts[0].deployment = None;
    let report = diff_snapshots(&candidate, &interface);
    assert!(report.changes.iter().any(|change| {
        change.category == CompatibilityCategory::ArtifactKindChanged
            && change.contract.as_deref() == Some("status")
    }));
    let reverse = diff_snapshots(&interface, &candidate);
    assert!(reverse.changes.iter().any(|change| {
        change.category == CompatibilityCategory::ArtifactKindChanged
            && change.contract.as_deref() == Some("status")
    }));
}

#[cfg(unix)]
#[test]
fn interface_crate_compiles_and_deployment_api_does_not() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    write_abi(root.path());
    fs::write(
        root.path().join("musubi.toml"),
        manifest("kind = \"interface\""),
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
    let output = root.path().join("sdk");
    generate(&tree, &output, CargoValidation::Required).unwrap();
    let private = crate::output_test_support::private_copy(&tree, root.path());
    let target = crate::output_test_support::cargo_target();
    let environment = [("CARGO_TARGET_DIR", target.as_path())];
    let success = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments: &["check", "--quiet"],
            current_dir: &private,
            environment: &environment,
        },
        CARGO_PROCESS_LIMITS,
    )
    .unwrap();
    assert!(success.status.success());

    fs::create_dir(private.join("src/bin")).unwrap();
    fs::write(
        private.join("src/bin/forbidden.rs"),
        "fn main() { let _ = generated_interface_sdk::status::Status::deployment; }\n",
    )
    .unwrap();
    let failure = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments: &["check", "--quiet", "--bin", "forbidden"],
            current_dir: &private,
            environment: &environment,
        },
        CARGO_PROCESS_LIMITS,
    )
    .unwrap();
    assert!(
        !failure.status.success(),
        "interface exposed deployment API"
    );
    assert!(
        crate::output_test_support::check(&tree, &output)
            .unwrap()
            .is_clean()
    );
}
