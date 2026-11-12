//! Version observations require no runtime, manifest, Git, compiler, or ambient environment.

use std::process::Command;

#[test]
fn embedded_version_human_and_json_agree_without_external_capabilities() {
    let root = tempfile::tempdir().unwrap();
    let run = |arguments: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_musubi"))
            .args(arguments)
            .current_dir(root.path())
            .env_clear()
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        output.stdout
    };
    let json = run(&["--format", "json", "--version"]);
    let info: musubi_core::GeneratorInfo = serde_json::from_slice(&json).unwrap();
    info.validate().unwrap();
    assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    let human = String::from_utf8(run(&["--version"])).unwrap();
    let banner = include_str!("../assets/version-banner.txt");
    assert!(banner.is_ascii());
    assert!(banner.ends_with('\n'));
    assert!(banner.lines().all(|line| !line.ends_with(' ')));
    assert!(human.starts_with(banner));
    assert_eq!(human.matches(banner).count(), 1);
    assert!(human.contains(&format!("musubi {}", info.version)));
    if let Some(commit) = &info.git_commit {
        assert!(human.contains(commit.as_ref()));
    }
    for expected in [
        &info.version,
        &info.build.rustc,
        &info.build.target,
        &info.build.profile,
    ] {
        assert!(human.contains(expected.as_ref()));
    }
    assert_eq!(json, run(&["--format=json", "--version"]));
    assert_eq!(json, run(&["--format", "json", "-V"]));
    assert_eq!(human.as_bytes(), run(&["-V"]));
    assert_eq!(root.path().read_dir().unwrap().count(), 0);
}

#[test]
fn root_help_and_no_arguments_share_the_brand_banner() {
    let banner = include_str!("../assets/version-banner.txt");
    for arguments in [vec![], vec!["--help"], vec!["-h"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_musubi"))
            .args(arguments)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.starts_with(banner));
        assert_eq!(help.matches(banner).count(), 1);
        assert!(help.contains("Usage: musubi"));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_musubi"))
        .args(["generate", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!String::from_utf8(output.stdout).unwrap().contains(banner));
}
