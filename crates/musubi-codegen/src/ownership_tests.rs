//! Closed ownership grammar, exact-byte binding and assembly budget regressions.

use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    GENERATED_MARKER, GeneratedFile, GeneratedTree, HardLimits, MAX_MARKER_BYTES, OwnershipError,
    parse_ownership_marker, validate_owned_files,
};

fn files() -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::from([
        (PathBuf::from("Cargo.toml"), b"manifest\n".to_vec()),
        (PathBuf::from("src/lib.rs"), b"// final bytes\n".to_vec()),
    ]);
    let marker = crate::ownership::render_marker(&files, MAX_MARKER_BYTES).unwrap();
    files.insert(GENERATED_MARKER.into(), marker);
    files
}

fn marker() -> serde_json::Value {
    serde_json::from_slice(&files()[&PathBuf::from(GENERATED_MARKER)]).unwrap()
}

fn parse(value: &serde_json::Value) -> Result<crate::OwnershipManifest, OwnershipError> {
    parse_ownership_marker(&serde_json::to_vec(value).unwrap(), HardLimits::V1)
}

#[test]
fn ownership_closed_schema_rejects_every_noncanonical_field_representation() {
    assert!(parse(&marker()).is_ok());
    for key in ["schema_version", "layout", "files"] {
        let mut value = marker();
        value.as_object_mut().unwrap().remove(key);
        assert!(parse(&value).is_err(), "missing {key}");
    }
    for key in ["path", "bytes", "sha256"] {
        let mut value = marker();
        value["files"][0].as_object_mut().unwrap().remove(key);
        assert!(parse(&value).is_err(), "missing record {key}");
    }
    for (key, invalid) in [
        ("bytes", serde_json::json!(true)),
        ("bytes", serde_json::json!(-1)),
        ("bytes", serde_json::json!(1.0)),
        ("bytes", serde_json::json!("1")),
        ("sha256", serde_json::json!("A".repeat(64))),
        ("sha256", serde_json::json!("0".repeat(63))),
    ] {
        let mut value = marker();
        value["files"][0][key] = invalid;
        assert!(parse(&value).is_err(), "invalid {key}");
    }
    for pointer in ["", "/files/0"] {
        let mut value = marker();
        value.pointer_mut(pointer).unwrap()["unknown"] = serde_json::json!(0);
        assert!(parse(&value).is_err());
    }
    let source = String::from_utf8(files()[&PathBuf::from(GENERATED_MARKER)].clone()).unwrap();
    for duplicate in [
        source.replacen('{', "{\"layout\":\"musubi/semantic-layout-v1\",", 1),
        source.replacen("\"path\":", "\"bytes\":0,\"path\":", 1),
        source.replace("\"bytes\":9", "\"bytes\":18446744073709551616"),
        "musubi/generated-v1\n".into(),
    ] {
        assert!(parse_ownership_marker(duplicate.as_bytes(), HardLimits::V1).is_err());
    }
    for key in ["schema_version", "layout"] {
        let mut value = marker();
        value[key] = serde_json::json!("unknown");
        assert!(parse(&value).is_err());
    }
}

#[test]
fn ownership_paths_are_sorted_normal_and_casefold_unique_including_ancestors() {
    for path in [
        "",
        "/absolute",
        "a/",
        "a//b",
        "./a",
        "a/../b",
        "a\\b",
        "a\n",
        "a\u{0085}",
        GENERATED_MARKER,
    ] {
        let mut value = marker();
        value["files"][0]["path"] = serde_json::json!(path);
        assert!(parse(&value).is_err(), "accepted {path:?}");
    }
    let mut value = marker();
    value["files"].as_array_mut().unwrap().reverse();
    assert!(parse(&value).is_err());
    for (first, second) in [
        ("a", "a"),
        ("A", "a"),
        ("A/x", "a/y"),
        ("Straße/x", "strasse/y"),
        ("a", "a/child"),
        (".MUSUBI-GENERATED/x", "src/lib.rs"),
    ] {
        let mut value = marker();
        value["files"][0]["path"] = serde_json::json!(first);
        value["files"][1]["path"] = serde_json::json!(second);
        assert!(parse(&value).is_err(), "accepted {first:?}, {second:?}");
    }
    for path in ["x".repeat(256), format!("{}leaf", "d/".repeat(65))] {
        let mut value = marker();
        value["files"][1]["path"] = serde_json::json!(path);
        assert!(parse(&value).is_err());
    }
}

#[test]
fn ownership_binds_every_file_without_normalizing_retained_bytes() {
    let original = files();
    validate_owned_files(&original, HardLimits::V1).unwrap();
    for mutation in ["changed", "removed", "extra", "same-size", "non-normal"] {
        let mut actual = original.clone();
        match mutation {
            "changed" => {
                actual
                    .get_mut(&PathBuf::from("src/lib.rs"))
                    .unwrap()
                    .push(b' ');
            }
            "same-size" => {
                actual.get_mut(&PathBuf::from("src/lib.rs")).unwrap()[0] = b'#';
            }
            "removed" => {
                actual.remove(&PathBuf::from("src/lib.rs"));
            }
            "extra" => {
                actual.insert("extra".into(), vec![]);
            }
            "non-normal" => {
                let bytes = actual.remove(&PathBuf::from("src/lib.rs")).unwrap();
                actual.insert("src//lib.rs".into(), bytes);
            }
            _ => unreachable!(),
        }
        assert!(
            validate_owned_files(&actual, HardLimits::V1).is_err(),
            "{mutation}"
        );
        assert!(
            GeneratedTree::from_files(
                actual
                    .into_iter()
                    .map(|(path, contents)| GeneratedFile { path, contents })
            )
            .is_err(),
            "{mutation}"
        );
    }
    let retained = GeneratedTree::from_files(
        original
            .clone()
            .into_iter()
            .map(|(path, contents)| GeneratedFile { path, contents }),
    )
    .unwrap();
    assert_eq!(
        retained
            .files()
            .map(|file| (file.path, file.contents))
            .collect::<BTreeMap<_, _>>(),
        original
    );
    assert!(GeneratedTree::default().validate_ownership().is_err());
}

#[test]
fn ownership_reserves_exact_marker_bytes_and_enforces_effective_limits() {
    let mut tree = GeneratedTree::default();
    tree.insert("src/lib.rs", b"// final bytes\n   ").unwrap();
    let reserved = HardLimits::V1.generated_bytes - tree.remaining_bytes();
    tree.seal().unwrap();
    let actual: usize = tree.file_entries().map(|(_, bytes)| bytes.len()).sum();
    assert_eq!(reserved, actual);
    let limits = HardLimits {
        generated_bytes: actual,
        generated_files: 2,
        ..HardLimits::V1
    };
    let mut exact = GeneratedTree::with_limits(limits);
    exact.insert("src/lib.rs", b"// final bytes\n").unwrap();
    assert!(exact.insert("extra", []).is_err());
    exact.seal().unwrap();
    let mut short = GeneratedTree::with_limits(HardLimits {
        generated_bytes: actual - 1,
        ..limits
    });
    assert!(short.insert("src/lib.rs", b"// final bytes\n").is_err());
    let mut deep = GeneratedTree::with_limits(HardLimits {
        generated_files: 2,
        ..HardLimits::V1
    });
    assert!(deep.insert("a/b/c/value", []).is_err());
    let bytes = vec![b' '; MAX_MARKER_BYTES + 1];
    assert_eq!(
        parse_ownership_marker(&bytes, HardLimits::V1),
        Err(OwnershipError::LimitExceeded)
    );
    let marker = files().remove(&PathBuf::from(GENERATED_MARKER)).unwrap();
    assert!(
        parse_ownership_marker(
            &marker,
            HardLimits {
                generated_files: 2,
                ..HardLimits::V1
            }
        )
        .is_err()
    );
    assert!(
        parse_ownership_marker(
            &marker,
            HardLimits {
                generated_bytes: marker.len(),
                ..HardLimits::V1
            }
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn ownership_rejects_non_utf8_retained_paths_on_every_unix_filesystem() {
    use std::os::unix::ffi::OsStringExt as _;
    let mut files = files();
    files.insert(
        std::ffi::OsString::from_vec(b"invalid-\xff".to_vec()).into(),
        vec![],
    );
    assert_eq!(
        validate_owned_files(&files, HardLimits::V1),
        Err(OwnershipError::InvalidPath)
    );
}

#[test]
fn ownership_hard_file_and_marker_boundaries_include_the_marker_itself() {
    let mut tree = GeneratedTree::default();
    for index in 0..HardLimits::V1.generated_files - 1 {
        tree.insert(format!("owners/value_{index:05}.rs"), b"// owner\n")
            .unwrap();
    }
    assert!(tree.insert("owners/overflow.rs", []).is_err());
    let reserved = HardLimits::V1.generated_bytes - tree.remaining_bytes();
    tree.seal().unwrap();
    assert_eq!(tree.file_entries().count(), 10_000);
    assert_eq!(
        tree.file_entries()
            .map(|(_, bytes)| bytes.len())
            .sum::<usize>(),
        reserved
    );
    tree.validate_ownership().unwrap();

    let mut exact = files().remove(&PathBuf::from(GENERATED_MARKER)).unwrap();
    exact.resize(MAX_MARKER_BYTES, b' ');
    parse_ownership_marker(&exact, HardLimits::V1).unwrap();
    exact.push(b' ');
    assert_eq!(
        parse_ownership_marker(&exact, HardLimits::V1),
        Err(OwnershipError::LimitExceeded)
    );
}
