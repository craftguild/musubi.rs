//! Exact manifest bytes are independent evidence shared by JSON and generated Rust.

use crate::{build_ir, load_inputs, render};
use musubi_core::sha256;
use std::{fs, path::Path};

#[test]
fn exact_manifest_digest_survives_ir_and_agrees_with_rust_and_release_json() {
    let root = tempfile::tempdir().unwrap();
    let manifest = root.path().join("musubi.toml");
    let raw = b"schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n[package]\nname = \"provenance-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[generation]\noutput = \"sdk\"\n[[contracts]]\nid = \"api\"\nkind = \"interface\"\nabi = \"abi.json\"\n";
    fs::write(&manifest, raw).unwrap();
    fs::write(root.path().join("abi.json"), b"[]").unwrap();
    let original = load_inputs(&manifest).unwrap();
    assert_eq!(original.manifest_sha256, sha256(raw));
    let ir = build_ir(original).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let output = root.path().join("inspection");
    fs::create_dir(&output).unwrap();
    for file in tree.files() {
        let path = output.join(file.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, file.contents).unwrap();
    }
    let inspected =
        musubi_testkit::inspect_generated_crate(&output.canonicalize().unwrap()).unwrap();
    assert_eq!(inspected.manifest_schema, "musubi/config-v4");
    assert_eq!(inspected.manifest_sha256, sha256(raw));
    assert_eq!(inspected.runtime, ir.runtime);
    assert_eq!(inspected.generator, crate::test_generator::info());
    let files = tree
        .files()
        .map(|file| (file.path, file.contents))
        .collect::<std::collections::BTreeMap<_, _>>();
    let release: serde_json::Value =
        serde_json::from_slice(&files[Path::new("musubi-release.json")]).unwrap();
    assert_eq!(release["schema_version"], "musubi/release-v4");
    assert_eq!(release["manifest_schema"], "musubi/config-v4");
    assert_eq!(
        release["manifest_sha256"],
        serde_json::to_value(sha256(raw)).unwrap()
    );
    assert_eq!(
        release["contracts"][0]["external_artifact_coherence"],
        "unverified"
    );
    let source = std::str::from_utf8(&files[Path::new("src/lib.rs")]).unwrap();
    assert!(source.contains("pub const MANIFEST_SCHEMA: &str = \"musubi/config-v4\";"));
    assert!(source.contains(&format!("pub const MANIFEST_SHA256: musubi_core::Sha256Digest = musubi_core::Sha256Digest::new({:?});", sha256(raw).as_ref())));
    assert!(!source.contains(root.path().to_str().unwrap()));

    fs::write(
        &manifest,
        [raw.as_slice(), b"# reviewed whitespace\n"].concat(),
    )
    .unwrap();
    let refreshed = build_ir(load_inputs(&manifest).unwrap()).unwrap();
    assert_eq!(
        ir.contracts[0].canonical_abi_sha256,
        refreshed.contracts[0].canonical_abi_sha256
    );
    assert_ne!(
        tree,
        render(&refreshed, &crate::test_generator::info()).unwrap()
    );
    let before = crate::build_snapshot(&ir, &crate::test_generator::info());
    let after = crate::build_snapshot(&refreshed, &crate::test_generator::info());
    let changes = crate::diff_snapshots(&before, &after).changes;
    assert_eq!(changes.len(), 1);
    assert_eq!(
        changes[0].category,
        crate::CompatibilityCategory::ProvenanceOnly
    );
    assert_eq!(changes[0].contract, None);
}
