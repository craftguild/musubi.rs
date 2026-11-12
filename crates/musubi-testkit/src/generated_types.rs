//! One public type definition per shared/event/error owner and exact facade re-exports.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use crate::{GeneratedCrateError, generated_tree::TreeSnapshot};

pub(crate) fn contract(
    snapshot: &TreeSnapshot,
    base: &Path,
    modules: &BTreeMap<String, Option<String>>,
) -> Result<(), GeneratedCrateError> {
    let mut expected = BTreeSet::new();
    let mut names = BTreeSet::new();
    for (module, path) in modules {
        let Some(path) = path else {
            continue;
        };
        if path == "artifact.rs" {
            continue;
        }
        let bytes = snapshot
            .files
            .get(&base.join(path))
            .ok_or(GeneratedCrateError::InvalidLayout)?;
        let types = definitions(bytes)?;
        let single = ["shared_types/", "events/", "errors/"]
            .iter()
            .any(|prefix| path.starts_with(prefix));
        if (single && types.len() != 1)
            || (path.starts_with("operations/") && types.len() > 2)
            || (path == "constructor.rs" && types.iter().any(|name| name != "ConstructorArgs"))
        {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        for name in types {
            if !names.insert(name.clone()) {
                return Err(GeneratedCrateError::InvalidLayout);
            }
            expected.insert((module.clone(), name));
        }
    }
    let source = snapshot
        .files
        .get(&base.join("types.rs"))
        .ok_or(GeneratedCrateError::InvalidLayout)?;
    if exports(source, true, false)? != expected {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    Ok(())
}

pub(crate) fn typed_exports(
    source: &[u8],
    expected: &BTreeSet<(String, String)>,
) -> Result<(), GeneratedCrateError> {
    if &exports(source, false, true)? != expected {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    Ok(())
}

pub(crate) fn definitions(source: &[u8]) -> Result<BTreeSet<String>, GeneratedCrateError> {
    let syntax = syntax(source)?;
    let mut names = BTreeSet::new();
    for item in syntax.items {
        let (name, visibility) = match item {
            syn::Item::Struct(item) => (item.ident, item.vis),
            syn::Item::Enum(item) => (item.ident, item.vis),
            syn::Item::Type(item) => (item.ident, item.vis),
            _ => continue,
        };
        if matches!(visibility, syn::Visibility::Public(_)) && !names.insert(name.to_string()) {
            return Err(GeneratedCrateError::InvalidLayout);
        }
    }
    Ok(names)
}

fn exports(
    source: &[u8],
    from_parent: bool,
    allow_modules: bool,
) -> Result<BTreeSet<(String, String)>, GeneratedCrateError> {
    let mut exports = BTreeSet::new();
    for item in syntax(source)?.items {
        if allow_modules && matches!(item, syn::Item::Mod(_)) {
            continue;
        }
        let syn::Item::Use(item) = item else {
            return Err(GeneratedCrateError::InvalidLayout);
        };
        if !matches!(item.vis, syn::Visibility::Public(_))
            || item.leading_colon.is_some()
            || !item.attrs.is_empty()
        {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        let tree = if from_parent {
            let syn::UseTree::Path(path) = item.tree else {
                return Err(GeneratedCrateError::InvalidLayout);
            };
            if path.ident != "super" {
                return Err(GeneratedCrateError::InvalidLayout);
            }
            *path.tree
        } else {
            item.tree
        };
        let syn::UseTree::Path(module) = tree else {
            return Err(GeneratedCrateError::InvalidLayout);
        };
        let syn::UseTree::Name(name) = *module.tree else {
            return Err(GeneratedCrateError::InvalidLayout);
        };
        if !exports.insert((module.ident.to_string(), name.ident.to_string())) {
            return Err(GeneratedCrateError::InvalidLayout);
        }
    }
    Ok(exports)
}

fn syntax(source: &[u8]) -> Result<syn::File, GeneratedCrateError> {
    syn::parse_file(std::str::from_utf8(source).map_err(|_| GeneratedCrateError::InvalidLayout)?)
        .map_err(|_| GeneratedCrateError::InvalidLayout)
}
