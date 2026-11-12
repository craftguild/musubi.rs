//! Semantic co-location, stable paths, and independent leaf bytes across ABI edits.

use std::{collections::BTreeMap, fs, path::Path};

use serde_json::{Value, json};

use crate::{GeneratedTree, build_ir, load_inputs, render};

fn tree(abi: &[Value]) -> GeneratedTree {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("abi.json"),
        serde_json::to_vec(abi).unwrap(),
    )
    .unwrap();
    fs::write(root.path().join("musubi.toml"), concat!(
        "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n",
        "[generation]\noutput = \"sdk\"\n[package]\nname = \"semantic-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        "[[contracts]]\nid = \"Vault\"\nkind = \"interface\"\nabi = \"abi.json\"\n"
    )).unwrap();
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    render(&ir, &crate::test_generator::info()).unwrap()
}

fn function(name: &str) -> Value {
    json!({"type":"function", "name":name, "stateMutability":"view", "inputs":[
        {"name":"record","type":"tuple","components":[{"name":"amount","type":"uint256"}]}],
        "outputs":[{"name":"value","type":"address"}]})
}

fn baseline() -> Vec<Value> {
    vec![
        function("read"),
        function("write"),
        json!({"type":"event","name":"Seen","anonymous":false,"inputs":[{"name":"value","type":"address","indexed":true}]}),
        json!({"type":"error","name":"Denied","inputs":[{"name":"value","type":"address"}]}),
    ]
}

fn leaves(tree: &GeneratedTree) -> BTreeMap<String, Vec<u8>> {
    tree.files()
        .filter_map(|file| {
            let path = file.path.to_str().unwrap().to_owned();
            ["operations", "events", "errors", "shared_types"]
                .iter()
                .any(|kind| path.contains(&format!("/{kind}/")))
                .then_some((path, file.contents))
        })
        .collect()
}

#[test]
fn semantic_layout_co_locates_each_owner_and_keeps_types_facade() {
    let tree = tree(&baseline());
    let leaves = leaves(&tree);
    assert_eq!(leaves.len(), 5); // Two functions share exactly one owned tuple.
    for (path, bytes) in &leaves {
        let source = std::str::from_utf8(bytes).unwrap();
        assert!(!source.contains("function_0") && !source.contains("function_1"));
        assert!(!source.contains("MANIFEST_SHA256") && !source.contains("crate::generator()"));
        if path.contains("/operations/") {
            assert_eq!(source.matches("pub struct ").count(), 2);
            assert!(
                source.contains("impl Vault {") && source.contains("impl<B: Clone> VaultClient<B>")
            );
            assert!(source.contains("fn descriptor()") && source.contains("fn abi_encode("));
        } else if path.contains("/events/") {
            assert!(source.contains("pub struct Seen") && source.contains("fn descriptor()"));
            assert!(source.contains("pub fn decode(") && source.contains("pub fn encode_log("));
        } else if path.contains("/errors/") {
            assert!(source.contains("pub struct Denied") && source.contains("fn descriptor()"));
        }
    }
    let facade = tree
        .files()
        .find(|file| file.path == Path::new("src/vault/types.rs"))
        .unwrap();
    let source = std::str::from_utf8(&facade.contents).unwrap();
    assert_eq!(source.matches("pub use ").count(), 7);
    assert!(!source.contains("pub struct ") && !source.contains("impl "));
    assert!(!tree.files().any(|file| file.path.ends_with("abi.rs")));
}

#[test]
fn semantic_layout_independent_add_remove_change_and_order_preserve_leaf_bytes() {
    let original = baseline();
    let first = tree(&original);
    assert_eq!(first, tree(&original));
    let expected = leaves(&first);
    let mut reversed = original.clone();
    reversed.reverse();
    assert_eq!(leaves(&tree(&reversed)), expected);
    let mut extended = original.clone();
    extended.push(function("aaaBeforeExisting"));
    let added = leaves(&tree(&extended));
    assert_eq!(added.len(), expected.len() + 1);
    for (path, bytes) in &expected {
        assert_eq!(added.get(path), Some(bytes), "{path}");
    }
    // Removing that independent owner reconstructs every old path and byte.
    extended.pop();
    assert_eq!(leaves(&tree(&extended)), expected);
    let mut changed = original;
    changed[0]["outputs"][0]["type"] = "bytes32".into();
    let changed = leaves(&tree(&changed));
    let different = expected
        .iter()
        .filter(|(path, bytes)| changed.get(*path) != Some(*bytes))
        .map(|(path, _)| path)
        .collect::<Vec<_>>();
    assert_eq!(different.len(), 1);
    assert!(different[0].contains("/operations/read_"));
}

#[test]
fn semantic_layout_real_shared_dependency_changes_only_its_users() {
    let original = baseline();
    let expected = leaves(&tree(&original));
    let mut changed = original;
    changed[0]["inputs"][0]["components"][0]["name"] = "balance".into();
    let actual = leaves(&tree(&changed));
    assert_eq!(actual.len(), expected.len() + 1); // New owned surface; write still uses the old one.
    for (path, bytes) in expected {
        if path.contains("/operations/read_") {
            assert_ne!(actual[&path], bytes);
        } else {
            assert_eq!(actual[&path], bytes, "{path}");
        }
    }
}

#[test]
fn semantic_layout_overload_keeps_private_path_with_explicit_public_name_change() {
    let base = [function("read")];
    let before = leaves(&tree(&base));
    let mut overload = function("read");
    overload["inputs"] = json!([]);
    let after = leaves(&tree(&[base[0].clone(), overload]));
    for (path, bytes) in &before {
        assert!(after.contains_key(path));
        if path.contains("/operations/") {
            assert_ne!(after[path], *bytes); // Existing public overload suffix policy is retained.
        } else {
            assert_eq!(after[path], *bytes);
        }
    }
}

#[test]
fn semantic_layout_typed_data_locality_tracks_only_real_graph_dependencies() {
    use musubi_core::{TypedDataField, TypedDataRoot, TypedDataStruct, TypedDataType};
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ir = build_ir(load_inputs(&workspace.join("safe-v141-sdk.toml")).unwrap()).unwrap();
    let mut typed = ir.typed_data[0].clone();
    let rendered = |typed: &crate::TypedDataIr| {
        let mut tree = GeneratedTree::default();
        crate::render_typed_data::typed_data_tree(&mut tree, typed).unwrap();
        tree.files()
            .filter(|file| file.path.parent().unwrap().ends_with("types"))
            .map(|file| (file.path, file.contents))
            .collect::<BTreeMap<_, _>>()
    };
    let before = rendered(&typed);
    typed.types.push(TypedDataStruct {
        name: "Child".into(),
        fields: vec![TypedDataField {
            name: "count".into(),
            ty: TypedDataType::Uint(256),
        }],
    });
    typed.roots.push(TypedDataRoot::StructHash {
        r#type: "Child".into(),
    });
    let independent = rendered(&typed);
    for (path, bytes) in &before {
        assert_eq!(independent.get(path), Some(bytes));
    }
    let safe_tx = typed
        .types
        .iter_mut()
        .find(|item| item.name == "SafeTx")
        .unwrap();
    safe_tx.fields.push(TypedDataField {
        name: "child".into(),
        ty: TypedDataType::Struct("Child".into()),
    });
    let dependent = rendered(&typed);
    typed
        .types
        .iter_mut()
        .find(|item| item.name == "Child")
        .unwrap()
        .fields[0]
        .name = "total".into();
    let updated = rendered(&typed);
    let changed = dependent
        .iter()
        .filter(|(path, bytes)| updated.get(*path) != Some(*bytes))
        .collect::<Vec<_>>();
    assert_eq!(changed.len(), 2);
    for (_, bytes) in changed {
        let source = std::str::from_utf8(bytes).unwrap();
        assert!(source.contains("pub struct Child") || source.contains("pub struct SafeTx"));
    }
}

#[test]
fn semantic_layout_many_owners_fit_the_hard_file_limit_and_one_more_fails_before_io() {
    use std::fmt::Write as _;

    use crate::{DiagnosticCode, HardLimits};

    let root = tempfile::tempdir().unwrap();
    let mut abi = (0..3328)
        .map(|index| {
            json!({"type":"function", "name":format!("f{index}"),
            "stateMutability":"view", "inputs":[], "outputs":[]})
        })
        .collect::<Vec<_>>();
    let mut manifest = concat!(
        "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n",
        "[generation]\noutput = \"sdk\"\n[package]\nname = \"many-owners-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    ).to_owned();
    for index in 0..3 {
        fs::write(
            root.path().join(format!("abi{index}.json")),
            serde_json::to_vec(&abi).unwrap(),
        )
        .unwrap();
        writeln!(
            manifest,
            "[[contracts]]\nid = \"c{index}\"\nkind = \"interface\"\nabi = \"abi{index}.json\""
        )
        .unwrap();
    }
    let manifest_path = root.path().join("musubi.toml");
    fs::write(&manifest_path, &manifest).unwrap();
    let mut ir = build_ir(load_inputs(&manifest_path).unwrap()).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    // 9,984 operation leaves + three sets of four contract files + four root files.
    assert_eq!(tree.file_entries().count(), HardLimits::V1.generated_files);
    tree.validate_ownership().unwrap();
    assert!(!root.path().join("sdk").exists());
    ir.limits.generated_files -= 1;
    assert_eq!(
        render(&ir, &crate::test_generator::info())
            .unwrap_err()
            .diagnostic
            .code,
        DiagnosticCode::LimitExceeded
    );
    abi.push(
        json!({"type":"function", "name":"oneMoreOwner", "stateMutability":"view",
                    "inputs":[], "outputs":[]}),
    );
    fs::write(
        root.path().join("abi2.json"),
        serde_json::to_vec(&abi).unwrap(),
    )
    .unwrap();
    let ir = build_ir(load_inputs(&manifest_path).unwrap()).unwrap();
    assert_eq!(
        render(&ir, &crate::test_generator::info())
            .unwrap_err()
            .diagnostic
            .code,
        DiagnosticCode::LimitExceeded
    );
    assert!(!root.path().join("sdk").exists());
    assert_eq!(fs::read_to_string(manifest_path).unwrap(), manifest);
}

#[test]
fn semantic_layout_marker_uses_string_order_for_valid_dollar_contract_ids() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("abi.json"), "[]").unwrap();
    fs::write(root.path().join("musubi.toml"), concat!(
        "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n",
        "[generation]\noutput = \"sdk\"\n[package]\nname = \"dollar-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        "[[contracts]]\nid = \"foo\"\nkind = \"interface\"\nabi = \"abi.json\"\n",
        "[[contracts]]\nid = \"foo$\"\nkind = \"interface\"\nabi = \"abi.json\"\n",
    )).unwrap();
    let ir = build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let marker = tree
        .file_entries()
        .find(|(path, _)| *path == Path::new(crate::GENERATED_MARKER))
        .unwrap()
        .1;
    let marker = crate::parse_ownership_marker(marker, crate::HardLimits::V1).unwrap();
    let assets = marker
        .files
        .iter()
        .filter(|file| file.path.starts_with("assets/"))
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(assets, ["assets/foo$/abi.json", "assets/foo/abi.json"]);
    tree.validate_ownership().unwrap();
}
