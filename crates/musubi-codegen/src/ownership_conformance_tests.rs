//! Shared Rust/Python vectors and the complete pinned Unicode default casefold mapping.

use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    marker: String,
    files: Vec<File>,
    valid: bool,
    marker_valid: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    path: String,
    contents: String,
}

#[test]
fn ownership_shared_python_rust_marker_and_tree_vectors_agree() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases: Vec<Case> = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/ownership/cases.json")).unwrap(),
    )
    .unwrap();
    assert!(cases.len() >= 60);
    for case in cases {
        let marker = case.marker.into_bytes();
        assert_eq!(
            crate::parse_ownership_marker(&marker, crate::HardLimits::V1).is_ok(),
            case.marker_valid,
            "{} marker",
            case.name
        );
        let mut files = case
            .files
            .into_iter()
            .map(|file| (PathBuf::from(file.path), file.contents.into_bytes()))
            .collect::<BTreeMap<_, _>>();
        files.insert(crate::GENERATED_MARKER.into(), marker);
        assert_eq!(
            crate::validate_owned_files(&files, crate::HardLimits::V1).is_ok(),
            case.valid,
            "{} tree",
            case.name
        );
    }
}

#[test]
fn ownership_unicode_matches_every_reviewed_python_table_mapping() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/vendor/unicode-16.0.0/CaseFolding.txt");
    let bytes = fs::read(path).unwrap();
    assert_eq!(
        hex::encode(musubi_core::sha256(&bytes)),
        "6f1f9c588eb4a5c718d9e8f93b782685e5c7fec872cf05e8e6878053599e09bb"
    );
    let mut count = 0;
    for line in std::str::from_utf8(&bytes).unwrap().lines() {
        let fields = line
            .split('#')
            .next()
            .unwrap()
            .split(';')
            .map(str::trim)
            .collect::<Vec<_>>();
        if fields.len() < 3 || !matches!(fields[1], "C" | "F") {
            continue;
        }
        let scalar = |value| char::from_u32(u32::from_str_radix(value, 16).unwrap()).unwrap();
        let character = scalar(fields[0]).to_string();
        let expected: String = fields[2].split_whitespace().map(scalar).collect();
        assert_eq!(
            caseless::default_case_fold_str(&character),
            expected,
            "{}",
            fields[0]
        );
        count += 1;
    }
    assert!(count > 1500);
}
