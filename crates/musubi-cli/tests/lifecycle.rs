//! Consumer lifecycle parsing, init isolation, old-schema rejection, and diagnostic redaction.

use std::{
    fmt::Write as _,
    fs,
    path::Path,
    process::{Command, Output},
};

const BINARY: &str = env!("CARGO_BIN_EXE_musubi");

fn command(root: &Path, arguments: &[&str]) -> Output {
    Command::new(BINARY)
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap()
}

fn fixture(root: &Path, contracts: usize) {
    fs::write(root.join("abi.json"), b"[]").unwrap();
    let mut manifest = String::from(
        "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \".\"\n[package]\nname = \"cli-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[generation]\noutput = \"sdk\"\n",
    );
    for index in 0..contracts {
        writeln!(
            manifest,
            "[[contracts]]\nid = \"item_{index}\"\nkind = \"interface\"\nabi = \"abi.json\""
        )
        .unwrap();
    }
    fs::write(root.join("musubi.toml"), manifest).unwrap();
}

#[test]
fn init_needs_neither_publisher_secrets_nor_external_executables() {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(BINARY)
        .current_dir(root.path())
        .env_clear()
        .env("PATH", "")
        .args(["--format", "json", "init"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(root.path().join("musubi.toml")).unwrap(),
        musubi_codegen::INITIAL_MANIFEST.as_bytes()
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    let again = command(root.path(), &["--format", "json", "init"]);
    assert!(!again.status.success());
    let generation = command(root.path(), &["--format", "json", "generate"]);
    assert!(!generation.status.success());
    let diagnostic: serde_json::Value = serde_json::from_slice(&generation.stderr).unwrap();
    assert_eq!(diagnostic["diagnostics"][0]["code"], "MSB-E001");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn overrides_are_rejected_equally_for_zero_one_and_multiple_entries() {
    for count in [0, 1, 2] {
        let root = tempfile::tempdir().unwrap();
        fixture(root.path(), count);
        let before = fs::read(root.path().join("musubi.toml")).unwrap();
        for operation in ["generate", "update", "check", "doctor"] {
            for flag in [
                "--out",
                "--abi",
                "--bytecode",
                "--creation-bytecode",
                "--runtime-bytecode",
                "--ast",
                "--metadata",
                "--storage-layout",
                "--contract-id",
                "--force",
                "--override",
                "--development-runtime-path",
                "--bundled-runtime",
                "--runtime-git-url",
                "--runtime-git-revision",
            ] {
                let output = command(
                    root.path(),
                    &["--format", "json", operation, flag, "credential-SECRET"],
                );
                assert_eq!(output.status.code(), Some(2));
                let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(diagnostic["diagnostics"][0]["code"], "MSB-E001");
                assert!(
                    !String::from_utf8(output.stderr)
                        .unwrap()
                        .contains("credential-SECRET")
                );
                assert!(output.stdout.is_empty());
            }
        }
        assert_eq!(fs::read(root.path().join("musubi.toml")).unwrap(), before);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }
}

#[test]
fn unsupported_schema_is_reported_before_identity_selection_or_output_writes() {
    for schema in ["musubi/config-v1", "musubi/config-v2", "musubi/config-v3"] {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("musubi.toml"),
            format!("schema_version = {schema:?}\n"),
        )
        .unwrap();
        fs::create_dir(root.path().join("sdk")).unwrap();
        fs::write(root.path().join("sdk/human"), b"preserve").unwrap();
        for operation in ["generate", "update", "check", "doctor"] {
            let output = command(root.path(), &["--format", "json", operation]);
            assert!(!output.status.success());
            let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(
                diagnostic["diagnostics"][0]["message"],
                "manifest schema is unsupported"
            );
        }
        assert_eq!(
            fs::read(root.path().join("sdk/human")).unwrap(),
            b"preserve"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }
}

#[test]
fn human_and_json_parse_failures_have_identical_redacted_facts() {
    let root = tempfile::tempdir().unwrap();
    for sensitive in [
        "/private/source-body-SECRET",
        "ssh://git:SECRET@host/repo",
        "SSH_AUTH_SOCK=SECRET",
        "-----BEGIN PRIVATE KEY-----SECRET",
    ] {
        let human = command(root.path(), &["generate", "--out", sensitive]);
        let json = command(
            root.path(),
            &["--format=json", "generate", "--out", sensitive],
        );
        let document: serde_json::Value = serde_json::from_slice(&json.stderr).unwrap();
        let human = String::from_utf8(human.stderr).unwrap();
        let message = document["diagnostics"][0]["message"].as_str().unwrap();
        assert!(human.contains(message));
        assert!(human.contains("MSB-E001"));
        assert!(!human.contains("SECRET"));
        assert!(!String::from_utf8(json.stderr).unwrap().contains("SECRET"));
    }
}

#[test]
fn rejected_manifest_paths_never_echo_unvalidated_values() {
    for sensitive in [
        "-----BEGIN PRIVATE KEY-----\nSECRET\n-----END PRIVATE KEY-----",
        "ssh://git:SECRET@host/repo",
        "../source-body-SECRET",
        "invalid\\SECRET",
        "SECRET source body with no path separators",
    ] {
        let root = tempfile::tempdir().unwrap();
        fixture(root.path(), 1);
        let path = root.path().join("musubi.toml");
        let original = fs::read_to_string(&path).unwrap();
        let edited = original.replace(
            "abi = \"abi.json\"",
            &format!("abi = {}", serde_json::to_string(sensitive).unwrap()),
        );
        fs::write(&path, &edited).unwrap();
        for format in ["human", "json"] {
            let output = command(root.path(), &["--format", format, "generate"]);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(stderr.contains("MSB-E002"));
            assert!(!stderr.contains("SECRET"), "unvalidated path leaked");
            assert!(!stderr.contains("PRIVATE KEY"));
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), edited);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }
}

#[test]
fn plausible_manifest_override_environment_does_not_change_snapshots() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), 2);
    assert!(
        command(root.path(), &["snapshot", "--out", "normal.json"])
            .status
            .success()
    );
    let mut injected = Command::new(BINARY);
    for name in [
        "MUSUBI_OUTPUT",
        "MUSUBI_GENERATION_OUTPUT",
        "MUSUBI_PACKAGE_NAME",
        "MUSUBI_ABI",
        "MUSUBI_TYPED_DATA",
        "MUSUBI_LIMITS",
        "MUSUBI_RUNTIME_GIT_URL",
        "MUSUBI_RUNTIME_GIT_REVISION",
        "MUSUBI_DEVELOPMENT_RUNTIME_PATH",
    ] {
        injected.env(name, "injected-SECRET");
    }
    let output = injected
        .current_dir(root.path())
        .args(["snapshot", "--out", "injected.json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read(root.path().join("normal.json")).unwrap(),
        fs::read(root.path().join("injected.json")).unwrap()
    );
}
