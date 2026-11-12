//! End-to-end deterministic generated crate and exact drift tests.

#[cfg(unix)]
use crate::output_test_support::{CargoValidation, check, generate};

use std::{fs, path::Path};

#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use tempfile::TempDir;

#[cfg(unix)]
use crate::{CARGO_PROCESS_LIMITS, ProcessRequest, run_bounded_process};
use crate::{build_ir, load_inputs, render};

fn fixture(root: &Path) {
    fs::write(
        root.join("abi.json"),
        br#"[
          {"type":"constructor","stateMutability":"nonpayable","inputs":[{"name":"initialOwner","type":"address"}]},
          {"type":"function","name":"owner","stateMutability":"view","inputs":[],"outputs":[{"name":"owner","type":"address"}]},
          {"type":"function","name":"deposit","stateMutability":"payable","inputs":[{"name":"amount","type":"uint256"}],"outputs":[]},
          {"type":"event","name":"Deposited","anonymous":false,"inputs":[{"name":"sender","type":"address","indexed":true},{"name":"memo","type":"string","indexed":true},{"name":"amount","type":"uint256","indexed":false}]},
          {"type":"error","name":"Unauthorized","inputs":[{"name":"caller","type":"address"}]}
        ]"#,
    )
    .unwrap();
    fs::write(root.join("creation.json"), br#""0x600060005560016000f3""#).unwrap();
    fs::write(
        root.join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[generation]
output = "sdk"

[package]
name = "generated-vault-sdk"
version = "0.1.0"
edition = "2024"

[[contracts]]
kind = "deployable"
id = "user_vault"
abi = "abi.json"
creation_bytecode = "creation.json"
"#,
    )
    .unwrap();
}

fn custom_fixture(root: &Path, id: &str, abi: &str) {
    fs::write(root.join("abi.json"), abi).unwrap();
    fs::write(root.join("creation.json"), br#""0x600060005560016000f3""#).unwrap();
    fs::write(
        root.join("musubi.toml"),
        format!(
            r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[generation]
output = "sdk"

[package]
name = "generated-name-sdk"
version = "0.1.0"
edition = "2024"

[[contracts]]
kind = "deployable"
id = "{id}"
abi = "abi.json"
creation_bytecode = "creation.json"
"#,
        ),
    )
    .unwrap();
}

#[cfg(unix)]
fn setup() -> (TempDir, crate::GeneratedTree) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    fs::create_dir_all(workspace.join("target")).unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    fixture(root.path());
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let ir = build_ir(inputs).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    (root, tree)
}

#[cfg(unix)]
fn assert_generated_rust_whitespace(tree: &crate::GeneratedTree) {
    for file in tree.files().filter(|file| {
        file.path
            .extension()
            .is_some_and(|extension| extension == "rs")
    }) {
        let source = std::str::from_utf8(&file.contents).unwrap();
        assert!(
            source.ends_with('\n'),
            "{} has no final newline",
            file.path.display()
        );
        assert!(
            !source.ends_with("\n\n"),
            "{} has a blank line at EOF",
            file.path.display()
        );
        for line in source[..source.len() - 1].split('\n') {
            assert_eq!(
                line.trim_end_matches([' ', '\t', '\r']),
                line,
                "{} has trailing whitespace",
                file.path.display()
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn generated_crate_compiles_and_is_byte_deterministic() {
    let (root, first) = setup();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let second = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
    assert_eq!(first, second);

    let output = root.path().join("sdk");
    generate(&first, &output, CargoValidation::Required).unwrap();
    assert!(check(&first, &output).unwrap().is_clean());
    crate::output_test_support::update(&second, &output, CargoValidation::Required).unwrap();
    assert!(check(&first, &output).unwrap().is_clean());
}

#[cfg(unix)]
#[test]
fn zero_field_generated_crate_passes_standalone_clippy_and_whitespace_checks() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    custom_fixture(
        root.path(),
        "status",
        r#"[
          {"type":"function","name":"version","stateMutability":"view","inputs":[],"outputs":[{"name":"value","type":"uint32"}]},
          {"type":"function","name":"emptyTuple","stateMutability":"view","inputs":[{"name":"input","type":"tuple","components":[]}],"outputs":[{"name":"output","type":"tuple","components":[]}]},
          {"type":"event","name":"Ping","anonymous":false,"inputs":[]},
          {"type":"error","name":"Paused","inputs":[]}
        ]"#,
    );
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
    assert_generated_rust_whitespace(&tree);
    let types = crate::output_test_support::rust_sources(&tree, "src/status");
    assert_eq!(types.matches("if !values.is_empty()").count(), 3);

    let output = root.path().join("sdk");
    generate(&tree, &output, CargoValidation::Required).unwrap();
    let private = crate::output_test_support::private_copy(&tree, root.path());
    let cargo_target = crate::output_test_support::cargo_target();
    let result = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments: &["clippy", "--all-targets", "--", "-D", "warnings"],
            current_dir: &private,
            environment: &[("CARGO_TARGET_DIR", &cargo_target)],
        },
        CARGO_PROCESS_LIMITS,
    )
    .unwrap();
    assert!(
        result.status.success(),
        "standalone Clippy failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        crate::output_test_support::check(&tree, &output)
            .unwrap()
            .is_clean()
    );
}

#[test]
fn git_tree_preserves_exact_ssh_identity() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let manifest_path = root.path().join("musubi.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap().replace(
        "source = \"path\"\nroot = \"../..\"",
        "source = \"git\"\nurl = \"ssh://git@github.com/craftguild/musubi.rs.git\"\nrevision = \"87553a40b014754de7991878c8e994490bad5939\"",
    );
    fs::write(&manifest_path, manifest).unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let tree = render(&build_ir(inputs).unwrap(), &crate::test_generator::info()).unwrap();
    let files = tree
        .files()
        .map(|file| (file.path, file.contents))
        .collect::<std::collections::BTreeMap<_, _>>();
    let cargo = String::from_utf8(files[Path::new("Cargo.toml")].clone()).unwrap();
    assert_eq!(
        cargo
            .matches("ssh://git@github.com/craftguild/musubi.rs.git")
            .count(),
        2
    );
    assert_eq!(
        cargo
            .matches("rev = \"87553a40b014754de7991878c8e994490bad5939\"")
            .count(),
        2
    );
    let release: serde_json::Value =
        serde_json::from_slice(&files[Path::new("musubi-release.json")]).unwrap();
    assert_eq!(release["runtime"]["source"], "git");
    assert_eq!(
        release["runtime"]["url"],
        "ssh://git@github.com/craftguild/musubi.rs.git"
    );
    assert_eq!(
        release["runtime"]["revision"],
        "87553a40b014754de7991878c8e994490bad5939"
    );
    let contract =
        String::from_utf8(files[Path::new("src/user_vault/artifact.rs")].clone()).unwrap();
    assert!(contract.contains("crate::runtime_identity()"));
    let root_source = String::from_utf8(files[Path::new("src/lib.rs")].clone()).unwrap();
    assert!(root_source.contains("87553a40b014754de7991878c8e994490bad5939"));
}

#[cfg(unix)]
#[test]
fn check_detects_added_removed_and_changed_files() {
    let (root, tree) = setup();
    let output = root.path().join("sdk");
    generate(&tree, &output, CargoValidation::Skip).unwrap();

    fs::write(output.join("assets/user_vault/abi.json"), b"changed").unwrap();
    fs::write(output.join("unexpected.txt"), b"unexpected").unwrap();
    fs::remove_file(output.join("src/user_vault/types.rs")).unwrap();
    let report = check(&tree, &output).unwrap();
    assert_eq!(
        report.changed,
        vec![PathBuf::from("assets/user_vault/abi.json")]
    );
    assert_eq!(report.removed, vec![PathBuf::from("unexpected.txt")]);
    assert_eq!(report.added, vec![PathBuf::from("src/user_vault/types.rs")]);
}

#[cfg(unix)]
#[test]
fn generation_refuses_markerless_directory() {
    let (root, tree) = setup();
    let output = root.path().join("sdk");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("human.txt"), b"preserve").unwrap();
    assert!(generate(&tree, &output, CargoValidation::Skip).is_err());
    assert_eq!(fs::read(output.join("human.txt")).unwrap(), b"preserve");
}

#[cfg(unix)]
#[test]
fn source_safe_keyword_names_generate_a_compiling_crate() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    custom_fixture(
        root.path(),
        "self",
        r#"[
          {"type":"function","name":"crate","stateMutability":"view","inputs":[{"name":"self","type":"uint256"}],"outputs":[{"name":"super","type":"uint256"}]},
          {"type":"function","name":"self","stateMutability":"nonpayable","inputs":[],"outputs":[]},
          {"type":"function","name":"_","stateMutability":"view","inputs":[{"name":"_","type":"uint256"}],"outputs":[]},
          {"type":"function","name":"$read","stateMutability":"view","inputs":[{"name":"$value","type":"uint256"}],"outputs":[]},
          {"type":"event","name":"type","anonymous":false,"inputs":[{"name":"crate","type":"address","indexed":false}]},
          {"type":"error","name":"match","inputs":[{"name":"self","type":"address"}]}
        ]"#,
    );
    let manifest = fs::read_to_string(root.path().join("musubi.toml")).unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        format!("{manifest}rust_module = \"_\"\n"),
    )
    .unwrap();
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    assert_eq!(ir.contracts[0].module, "underscore_contract");
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let source = crate::output_test_support::rust_sources(&tree, "src/underscore_contract");
    assert!(source.contains("pub struct SelfType;"));
    assert!(source.contains("pub fn crate_value("));
    assert!(source.contains("pub fn self_value("));
    assert!(source.contains("pub fn underscore_value("));
    assert!(source.contains("pub fn dollar_read("));
    generate(
        &tree,
        &root.path().join("keyword-sdk"),
        CargoValidation::Required,
    )
    .unwrap();
}

#[test]
fn normalized_field_collision_fails_before_rendering() {
    let root = tempfile::tempdir().unwrap();
    custom_fixture(
        root.path(),
        "vault",
        r#"[{"type":"function","name":"set","stateMutability":"nonpayable","inputs":[{"name":"fooBar","type":"uint256"},{"name":"foo_bar","type":"uint256"}],"outputs":[]}]"#,
    );
    let error = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap_err();
    assert_eq!(
        error.diagnostics.iter().next().unwrap().code,
        crate::DiagnosticCode::InconsistentArtifact
    );
}

#[test]
fn cross_kind_type_collision_fails_before_rendering() {
    let root = tempfile::tempdir().unwrap();
    custom_fixture(
        root.path(),
        "vault",
        r#"[
          {"type":"event","name":"Collision","anonymous":false,"inputs":[]},
          {"type":"error","name":"Collision","inputs":[]}
        ]"#,
    );
    let error = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap_err();
    assert_eq!(
        error.diagnostics.iter().next().unwrap().code,
        crate::DiagnosticCode::InconsistentArtifact
    );
}

#[test]
fn render_applies_downward_generated_output_limits_from_manifest() {
    for limit in ["generated_files = 1", "generated_bytes = 1", "nesting = 1"] {
        let root = tempfile::tempdir().unwrap();
        fixture(root.path());
        let manifest = fs::read_to_string(root.path().join("musubi.toml")).unwrap();
        fs::write(
            root.path().join("musubi.toml"),
            format!("{manifest}\n[limits]\n{limit}\n"),
        )
        .unwrap();
        let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
        assert!(render(&ir, &crate::test_generator::info()).is_err());
    }
}

#[cfg(unix)]
#[test]
fn rendered_and_actual_tree_share_the_lowered_depth_limit() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let manifest = fs::read_to_string(root.path().join("musubi.toml")).unwrap();
    fs::write(
        root.path().join("musubi.toml"),
        format!("{manifest}\n[limits]\nnesting = 3\n"),
    )
    .unwrap();
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let output = root.path().join("depth-sdk");
    generate(&tree, &output, CargoValidation::Skip).unwrap();
    assert!(check(&tree, &output).unwrap().is_clean());
}
