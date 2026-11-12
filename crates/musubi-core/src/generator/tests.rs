//! Closed metadata and honest unknown/dirty state regression tests.

use super::*;

fn fixture() -> GeneratorInfo {
    GeneratorInfo {
        schema_version: GENERATOR_INFO_SCHEMA.into(),
        version: "0.1.0".into(),
        revision: "unknown".into(),
        git_commit: None,
        dirty: None,
        build: GeneratorBuild {
            rustc: "rustc 1.85.0 (4d91de4e4 2025-02-17)".into(),
            host: "aarch64-apple-darwin".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "debug".into(),
        },
    }
}

#[test]
fn generator_info_roundtrip_requires_all_fields_and_rejects_unknown_fields() {
    let info = fixture();
    info.validate().unwrap();
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(
        serde_json::from_value::<GeneratorInfo>(json.clone()).unwrap(),
        info
    );
    for field in [
        "schema_version",
        "version",
        "revision",
        "git_commit",
        "dirty",
        "build",
    ] {
        let mut missing = json.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<GeneratorInfo>(missing).is_err(),
            "{field}"
        );
    }
    for field in ["rustc", "host", "target", "profile"] {
        let mut missing = json.clone();
        missing["build"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<GeneratorInfo>(missing).is_err());
    }
    let mut extra = json.clone();
    extra["extra"] = true.into();
    assert!(serde_json::from_value::<GeneratorInfo>(extra).is_err());
    let mut extra = json;
    extra["build"]["extra"] = true.into();
    assert!(serde_json::from_value::<GeneratorInfo>(extra).is_err());
}

#[test]
fn generator_info_requires_consistent_commit_dirty_and_display() {
    let commit = "046477cb8a19e8db4d93d7adb8b82e2ecec77577";
    let mut info = fixture();
    info.git_commit = Some(commit.into());
    assert!(info.validate().is_err());
    for dirty in [false, true] {
        info.dirty = Some(dirty);
        info.revision = if dirty {
            format!("{commit}-dirty").into()
        } else {
            commit.into()
        };
        info.validate().unwrap();
        info.dirty = Some(!dirty);
        assert!(info.validate().is_err());
    }
    for invalid in [
        "main",
        "1234",
        "0000000000000000000000000000000000000000",
        "046477CB8A19E8DB4D93D7ADB8B82E2ECEC77577",
    ] {
        info.git_commit = Some(invalid.into());
        info.revision = invalid.into();
        info.dirty = Some(false);
        assert!(info.validate().is_err());
    }
}

#[test]
fn generator_info_rejects_unbounded_or_noncanonical_text() {
    let original = serde_json::to_value(fixture()).unwrap();
    for pointer in [
        "/schema_version",
        "/version",
        "/revision",
        "/build/rustc",
        "/build/host",
        "/build/target",
        "/build/profile",
    ] {
        for invalid in ["", "/private/source", "a\nb", "user@host", &"x".repeat(257)] {
            let mut json = original.clone();
            *json.pointer_mut(pointer).unwrap() = invalid.into();
            assert!(
                serde_json::from_value::<GeneratorInfo>(json)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{pointer}"
            );
        }
    }
}
