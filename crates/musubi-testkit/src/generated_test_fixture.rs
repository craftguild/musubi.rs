//! Exact generated-tree construction shared by conformance self-tests.

use std::{fmt::Write as _, fs, path::Path};

use musubi_core::{keccak256, sha256};
use serde_json::Value;

const ROOT_PREAMBLE: &str = "//! Deterministically generated typed EVM contract SDK.\n\n\
#![forbid(unsafe_code)]\n#![warn(missing_docs)]\n\n\
musubi_runtime::assert_generated_abi!(2);\n\n";

pub(crate) fn materialize(root: &Path, release: &mut Value) {
    let _ = fs::remove_dir_all(root.join("src"));
    let _ = fs::remove_dir_all(root.join("assets"));
    fs::create_dir(root.join("src")).unwrap();
    let mut root_source = String::from(ROOT_PREAMBLE);
    root_source.push_str(&crate::generated_provenance::root_source(
        release["manifest_schema"].as_str().unwrap(),
        &serde_json::from_value(release["manifest_sha256"].clone()).unwrap(),
        &serde_json::from_value(release["runtime"].clone()).unwrap(),
        &serde_json::from_value(release["generator"].clone()).unwrap(),
    ));
    for contract in release["contracts"].as_array_mut().unwrap() {
        materialize_contract(root, contract, &mut root_source);
    }
    for typed in release["typed_data"].as_array().unwrap() {
        let id = typed["id"].as_str().unwrap();
        let module = typed["rust_module"].as_str().unwrap();
        writeln!(
            root_source,
            "/// Owned EIP-712 authority `{id}`.\n#[rustfmt::skip]\npub mod {module};"
        )
        .expect("String formatting cannot fail");
        let source = root.join("src").join(module);
        fs::create_dir(&source).unwrap();
        fs::create_dir(source.join("types")).unwrap();
        fs::write(source.join("descriptors.rs"), b"//! fixture\n").unwrap();
        let mut facade =
            String::from("//! fixture\n#[path = \"descriptors.rs\"]\nmod __evm_descriptors;\n");
        let mut names = std::collections::BTreeSet::new();
        for entry in typed["roots"].as_array().unwrap() {
            for key in ["primary_type", "domain_type"] {
                if let Some(name) = entry[key].as_str() {
                    names.insert(name);
                }
            }
        }
        for name in names {
            let identity = format!("{}:{id}:{name}", id.len());
            let owner = musubi_codegen::semantic_owner("types", name, identity.as_bytes()).unwrap();
            writeln!(
                facade,
                "#[path = {:?}]\nmod {};\npub use {}::{name};",
                owner.path, owner.module, owner.module
            )
            .expect("String formatting cannot fail");
            fs::write(source.join(owner.path), format!("//! Owned EIP-712 `{name}` value, conversion, and hashing methods.\n\npub struct {name};\n")).unwrap();
        }
        fs::write(source.join("mod.rs"), facade).unwrap();
    }
    fs::write(root.join("src/lib.rs"), root_source).unwrap();
    fs::write(
        root.join("musubi-release.json"),
        serde_json::to_vec(release).unwrap(),
    )
    .unwrap();
    reseal(root);
}

fn materialize_contract(root: &Path, contract: &mut Value, root_source: &mut String) {
    let id = contract["id"].as_str().unwrap().to_owned();
    let module = contract["rust_module"].as_str().unwrap().to_owned();
    writeln!(
        root_source,
        "/// Typed SDK for manifest contract `{id}`.\n#[rustfmt::skip]\npub mod {module};"
    )
    .expect("String formatting cannot fail");
    let assets = root.join("assets").join(&id);
    fs::create_dir_all(&assets).unwrap();
    let abi = b"[]";
    fs::write(assets.join("abi.json"), abi).unwrap();
    contract["canonical_abi_sha256"] = Value::String(hex::encode(sha256(abi)));
    let mut includes = vec![format!("../../assets/{id}/abi.json")];
    if !contract["creation_template_keccak256"].is_null() {
        let creation = b"creation";
        fs::write(assets.join("creation.bin"), creation).unwrap();
        contract["creation_template_keccak256"] = Value::String(hex::encode(keccak256(creation)));
        includes.push(format!("../../assets/{id}/creation.bin"));
    }
    if !contract["runtime_template_keccak256"].is_null() {
        let runtime = b"runtime";
        fs::write(assets.join("runtime.bin"), runtime).unwrap();
        contract["runtime_template_keccak256"] = Value::String(hex::encode(keccak256(runtime)));
        includes.push(format!("../../assets/{id}/runtime.bin"));
    }
    let source = root.join("src").join(&module);
    fs::create_dir_all(&source).unwrap();
    let module_source = includes
        .iter()
        .enumerate()
        .map(|(index, path)| format!("const ASSET_{index}: &[u8] = include_bytes!({path:?});\n"))
        .fold(String::from("//! fixture\n"), |mut output, line| {
            output.push_str(&line);
            output
        });
    fs::write(source.join("artifact.rs"), module_source).unwrap();
    let mut facade = String::from(
        "//! fixture\npub mod types;\n#[path = \"artifact.rs\"]\nmod __evm_artifact;\n",
    );
    if contract["kind"] == "deployable" {
        facade.push_str("#[path = \"constructor.rs\"]\nmod __evm_constructor;\n");
        fs::write(source.join("constructor.rs"), b"//! fixture\n").unwrap();
    }
    fs::write(source.join("mod.rs"), facade).unwrap();
    fs::write(source.join("types.rs"), b"//! fixture\n").unwrap();
}

/// Uses a canonical system temporary parent without hiding symlinks in inspected SDK paths.
pub(crate) fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

/// Test-only inventory repair makes negative source/release cases pass ownership first.
pub(crate) fn reseal(root: &Path) {
    let mut files = std::collections::BTreeMap::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                directories.push(entry.path());
            } else {
                assert!(kind.is_file());
                let path = entry.path().strip_prefix(root).unwrap().to_path_buf();
                if path != Path::new(musubi_codegen::GENERATED_MARKER) {
                    files.insert(path, fs::read(entry.path()).unwrap());
                }
            }
        }
    }
    let records = files
        .into_iter()
        .map(|(path, bytes)| musubi_codegen::OwnedFile {
            path: path.to_str().unwrap().into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(sha256(&bytes)),
        })
        .collect();
    let marker = musubi_codegen::OwnershipManifest {
        schema_version: musubi_codegen::OWNERSHIP_SCHEMA.into(),
        layout: musubi_codegen::SEMANTIC_LAYOUT.into(),
        files: records,
    };
    fs::write(
        root.join(musubi_codegen::GENERATED_MARKER),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();
}
