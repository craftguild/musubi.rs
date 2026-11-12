//! Typed-data compatibility classification independent from EVM ABI wire changes.

use std::collections::{BTreeMap, BTreeSet};

use crate::{CompatibilityCategory, CompatibilityChange, CompatibilitySnapshot, TypedDataSnapshot};

pub(crate) fn classify(
    old: &CompatibilitySnapshot,
    new: &CompatibilitySnapshot,
) -> BTreeSet<CompatibilityChange> {
    let mut changes = BTreeSet::new();
    let old_entries = by_id(&old.typed_data);
    let new_entries = by_id(&new.typed_data);
    for id in old_entries
        .keys()
        .chain(new_entries.keys())
        .collect::<BTreeSet<_>>()
    {
        match (old_entries.get(*id), new_entries.get(*id)) {
            (Some(old), Some(new)) if surface_equal(old, new) => {
                if old != new {
                    add(&mut changes, CompatibilityCategory::ProvenanceOnly, id);
                }
            }
            (Some(old), Some(new)) if additive(old, new) => {
                add(&mut changes, CompatibilityCategory::TypedDataAdditive, id);
                let rust = if generated_methods_preserved(old, new) {
                    CompatibilityCategory::RustSourceAdditive
                } else {
                    CompatibilityCategory::RustSourceBreaking
                };
                add(&mut changes, rust, id);
            }
            (Some(_), Some(_) | None) => {
                add(&mut changes, CompatibilityCategory::TypedDataBreaking, id);
                add(&mut changes, CompatibilityCategory::RustSourceBreaking, id);
            }
            (None, Some(_)) => {
                add(&mut changes, CompatibilityCategory::TypedDataAdditive, id);
                add(&mut changes, CompatibilityCategory::RustSourceAdditive, id);
            }
            (None, None) => {}
        }
    }
    changes
}

fn surface_equal(old: &TypedDataSnapshot, new: &TypedDataSnapshot) -> bool {
    old.rust_module == new.rust_module
        && old.types == new.types
        && old.roots == new.roots
        && old.root_evidence == new.root_evidence
}

fn additive(old: &TypedDataSnapshot, new: &TypedDataSnapshot) -> bool {
    old.rust_module == new.rust_module
        && old.types.iter().all(|item| new.types.contains(item))
        && old.roots.iter().all(|item| new.roots.contains(item))
        && old
            .root_evidence
            .iter()
            .all(|item| new.root_evidence.contains(item))
}

fn generated_methods_preserved(old: &TypedDataSnapshot, new: &TypedDataSnapshot) -> bool {
    old.roots.iter().all(|root| {
        root.domain_type().is_none()
            || digest_method(root, &old.roots) == digest_method(root, &new.roots)
    })
}

fn digest_method(
    root: &musubi_core::TypedDataRoot,
    roots: &[musubi_core::TypedDataRoot],
) -> String {
    crate::names::typed_data_digest_method(root, roots).unwrap_or_else(|| "digest".into())
}

fn by_id(entries: &[TypedDataSnapshot]) -> BTreeMap<&str, &TypedDataSnapshot> {
    entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect()
}

fn add(changes: &mut BTreeSet<CompatibilityChange>, category: CompatibilityCategory, id: &str) {
    changes.insert(CompatibilityChange {
        category,
        contract: Some(id.into()),
    });
}
