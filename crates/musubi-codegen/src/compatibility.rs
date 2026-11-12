//! Compatibility classification across canonical snapshots.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{
    AbiField, AbiType, ConstructorDescriptor, ContractAbi, ErrorDescriptor, EventDescriptor,
    FunctionDescriptor,
};
use serde::{Deserialize, Serialize};

use crate::{
    CompatibilitySnapshot, ContractSnapshot,
    compatibility_surface::{rust_abi_view, rust_enrichment_view},
};

/// Stable compatibility classification.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityCategory {
    /// A contract changed between deployable and ABI-only interface capability classes.
    ArtifactKindChanged,
    /// Existing typed-data strings, hashes, roots, or generated source changed.
    TypedDataBreaking,
    /// Independent typed-data roots/types were added without changing existing identities.
    TypedDataAdditive,
    /// Existing EVM calls, logs, or errors are no longer wire-compatible.
    EvmWireBreaking,
    /// New EVM wire surface was added without changing existing wire identities.
    EvmWireAdditive,
    /// Existing generated Rust source no longer compiles unchanged.
    RustSourceBreaking,
    /// New generated Rust surface was added without renaming existing items.
    RustSourceAdditive,
    /// Creation/runtime template or patch layout changed.
    DeploymentIdentityChanged,
    /// Only package or external-input provenance changed.
    ProvenanceOnly,
}

/// One classified change, optionally scoped to a boundary identifier.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityChange {
    /// Stable category.
    pub category: CompatibilityCategory,
    /// Stable contract or typed-data identifier, or `None` for package-wide changes.
    pub contract: Option<String>,
}

/// Deterministically ordered compatibility report.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityReport {
    /// Unique changes sorted by category and contract identifier.
    pub changes: Vec<CompatibilityChange>,
}

impl CompatibilityReport {
    /// Reports whether the snapshots are compatibility-equivalent.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// Classifies all material changes between two validated snapshots.
#[must_use]
pub fn diff_snapshots(
    old: &CompatibilitySnapshot,
    new: &CompatibilitySnapshot,
) -> CompatibilityReport {
    let mut changes = BTreeSet::new();
    classify_package(old, new, &mut changes);
    let old_contracts = by_id(&old.contracts);
    let new_contracts = by_id(&new.contracts);
    for id in old_contracts
        .keys()
        .chain(new_contracts.keys())
        .collect::<BTreeSet<_>>()
    {
        match (old_contracts.get(*id), new_contracts.get(*id)) {
            (Some(old), Some(new)) => classify_contract(old, new, &mut changes),
            (None, Some(new)) => {
                add(&mut changes, CompatibilityCategory::EvmWireAdditive, id);
                add(&mut changes, CompatibilityCategory::RustSourceAdditive, id);
                if new.deployment.is_some() {
                    add(
                        &mut changes,
                        CompatibilityCategory::DeploymentIdentityChanged,
                        id,
                    );
                }
            }
            (Some(old), None) => {
                add(&mut changes, CompatibilityCategory::EvmWireBreaking, id);
                add(&mut changes, CompatibilityCategory::RustSourceBreaking, id);
                if old.deployment.is_some() {
                    add(
                        &mut changes,
                        CompatibilityCategory::DeploymentIdentityChanged,
                        id,
                    );
                }
            }
            (None, None) => {}
        }
    }
    changes.extend(crate::compatibility_typed_data::classify(old, new));
    CompatibilityReport {
        changes: changes.into_iter().collect(),
    }
}

fn classify_package(
    old: &CompatibilitySnapshot,
    new: &CompatibilitySnapshot,
    changes: &mut BTreeSet<CompatibilityChange>,
) {
    if old.package_name != new.package_name
        || old.generator_abi_version != new.generator_abi_version
        || old.runtime_abi_version != new.runtime_abi_version
    {
        changes.insert(CompatibilityChange {
            category: CompatibilityCategory::RustSourceBreaking,
            contract: None,
        });
    } else if old.package_version != new.package_version
        || old.generator != new.generator
        || old.runtime != new.runtime
        || old.manifest_schema != new.manifest_schema
        || old.manifest_sha256 != new.manifest_sha256
    {
        changes.insert(CompatibilityChange {
            category: CompatibilityCategory::ProvenanceOnly,
            contract: None,
        });
    }
}

fn classify_contract(
    old: &ContractSnapshot,
    new: &ContractSnapshot,
    changes: &mut BTreeSet<CompatibilityChange>,
) {
    let id = old.id.as_str();
    if old.kind != new.kind {
        add(changes, CompatibilityCategory::ArtifactKindChanged, id);
        add(changes, CompatibilityCategory::RustSourceBreaking, id);
        add(
            changes,
            CompatibilityCategory::DeploymentIdentityChanged,
            id,
        );
    }
    let wire = wire_delta(&old.abi, &new.abi);
    match wire {
        WireDelta::Breaking => {
            add(changes, CompatibilityCategory::EvmWireBreaking, id);
            add(changes, CompatibilityCategory::RustSourceBreaking, id);
        }
        WireDelta::Additive => {
            add(changes, CompatibilityCategory::EvmWireAdditive, id);
            let rust_category = if rust_addition_preserves_existing(old, new) {
                CompatibilityCategory::RustSourceAdditive
            } else {
                CompatibilityCategory::RustSourceBreaking
            };
            add(changes, rust_category, id);
        }
        WireDelta::Same if !rust_equal(old, new) => {
            add(changes, CompatibilityCategory::RustSourceBreaking, id);
        }
        WireDelta::Same => {}
    }
    if !deployment_equal(old.deployment.as_ref(), new.deployment.as_ref()) {
        add(
            changes,
            CompatibilityCategory::DeploymentIdentityChanged,
            id,
        );
    }
    let has_substantive = changes.iter().any(|change| {
        change.contract.as_deref() == Some(id)
            && change.category != CompatibilityCategory::ProvenanceOnly
    });
    if !has_substantive && old.provenance != new.provenance {
        add(changes, CompatibilityCategory::ProvenanceOnly, id);
    }
}

fn deployment_equal(
    old: Option<&crate::DeploymentSnapshot>,
    new: Option<&crate::DeploymentSnapshot>,
) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => {
            old.creation_template_keccak256 == new.creation_template_keccak256
                && old.creation_template_bytes == new.creation_template_bytes
                && old.creation_links == new.creation_links
                && runtime_deployment_equal(old.runtime.as_ref(), new.runtime.as_ref())
        }
        (None, None) => true,
        _ => false,
    }
}

fn runtime_deployment_equal(
    old: Option<&crate::RuntimeDeploymentSnapshot>,
    new: Option<&crate::RuntimeDeploymentSnapshot>,
) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => {
            old.template_keccak256 == new.template_keccak256
                && old.template_bytes == new.template_bytes
                && old.links == new.links
                && old.immutables == new.immutables
        }
        (None, None) => true,
        _ => false,
    }
}

fn rust_equal(old: &ContractSnapshot, new: &ContractSnapshot) -> bool {
    old.rust_module == new.rust_module
        && rust_enrichment_view(&old.enrichments) == rust_enrichment_view(&new.enrichments)
        && rust_abi_view(&old.abi, &old.enrichments) == rust_abi_view(&new.abi, &new.enrichments)
}

fn rust_addition_preserves_existing(old: &ContractSnapshot, new: &ContractSnapshot) -> bool {
    if old.rust_module != new.rust_module
        || rust_enrichment_view(&old.enrichments) != rust_enrichment_view(&new.enrichments)
    {
        return false;
    }
    let old = rust_abi_view(&old.abi, &old.enrichments);
    let new = rust_abi_view(&new.abi, &new.enrichments);
    old.constructor == new.constructor
        && old.fallback == new.fallback
        && old.receive == new.receive
        && exact_subset(&old.functions, &new.functions)
        && exact_subset(&old.events, &new.events)
        && exact_subset(&old.errors, &new.errors)
        && !introduces_overload(&old, &new)
}

fn introduces_overload(old: &ContractAbi, new: &ContractAbi) -> bool {
    new.functions.iter().any(|item| {
        !old.functions.contains(item) && old.functions.iter().any(|old| old.name == item.name)
    }) || new.events.iter().any(|item| {
        !old.events.contains(item) && old.events.iter().any(|old| old.name == item.name)
    }) || new.errors.iter().any(|item| {
        !old.errors.contains(item) && old.errors.iter().any(|old| old.name == item.name)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WireDelta {
    Same,
    Additive,
    Breaking,
}

fn wire_delta(old: &ContractAbi, new: &ContractAbi) -> WireDelta {
    let base_equal = constructor_eq(old.constructor.as_ref(), new.constructor.as_ref())
        && old.fallback == new.fallback
        && old.receive == new.receive;
    if !base_equal {
        return WireDelta::Breaking;
    }
    let old_in_new = subset_by(&old.functions, &new.functions, function_eq)
        && subset_by(&old.events, &new.events, event_eq)
        && subset_by(&old.errors, &new.errors, error_eq);
    if !old_in_new {
        return WireDelta::Breaking;
    }
    let new_in_old = subset_by(&new.functions, &old.functions, function_eq)
        && subset_by(&new.events, &old.events, event_eq)
        && subset_by(&new.errors, &old.errors, error_eq);
    if new_in_old {
        WireDelta::Same
    } else {
        WireDelta::Additive
    }
}

fn constructor_eq(
    left: Option<&ConstructorDescriptor>,
    right: Option<&ConstructorDescriptor>,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            left.state_mutability == right.state_mutability
                && fields_eq(&left.inputs, &right.inputs)
        }
        (None, None) => true,
        _ => false,
    }
}

fn function_eq(left: &FunctionDescriptor, right: &FunctionDescriptor) -> bool {
    left.signature == right.signature
        && left.selector == right.selector
        && left.state_mutability == right.state_mutability
        && fields_eq(&left.inputs, &right.inputs)
        && fields_eq(&left.outputs, &right.outputs)
}

fn event_eq(left: &EventDescriptor, right: &EventDescriptor) -> bool {
    left.signature == right.signature
        && left.topic == right.topic
        && left.anonymous == right.anonymous
        && left.inputs.len() == right.inputs.len()
        && left.inputs.iter().zip(&right.inputs).all(|(left, right)| {
            left.indexed == right.indexed && wire_type_eq(&left.field.ty, &right.field.ty)
        })
}

fn error_eq(left: &ErrorDescriptor, right: &ErrorDescriptor) -> bool {
    left.signature == right.signature
        && left.selector == right.selector
        && fields_eq(&left.inputs, &right.inputs)
}

fn fields_eq(left: &[AbiField], right: &[AbiField]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| wire_type_eq(&left.ty, &right.ty))
}

fn wire_type_eq(left: &AbiType, right: &AbiType) -> bool {
    match (left, right) {
        (AbiType::Address, AbiType::Address)
        | (AbiType::Bool, AbiType::Bool)
        | (AbiType::String, AbiType::String)
        | (AbiType::Bytes, AbiType::Bytes)
        | (AbiType::Function, AbiType::Function) => true,
        (AbiType::FixedBytes(left), AbiType::FixedBytes(right))
        | (AbiType::Uint(left), AbiType::Uint(right))
        | (AbiType::Int(left), AbiType::Int(right)) => left == right,
        (
            AbiType::UFixed(left_bits, left_decimals),
            AbiType::UFixed(right_bits, right_decimals),
        )
        | (AbiType::Fixed(left_bits, left_decimals), AbiType::Fixed(right_bits, right_decimals)) => {
            left_bits == right_bits && left_decimals == right_decimals
        }
        (AbiType::Array(left), AbiType::Array(right)) => wire_type_eq(left, right),
        (AbiType::FixedArray(left, left_length), AbiType::FixedArray(right, right_length)) => {
            left_length == right_length && wire_type_eq(left, right)
        }
        (AbiType::Tuple(left), AbiType::Tuple(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| wire_type_eq(&left.ty, &right.ty))
        }
        _ => false,
    }
}

fn subset_by<T>(old: &[T], new: &[T], equal: fn(&T, &T) -> bool) -> bool {
    old.iter().all(|old| new.iter().any(|new| equal(old, new)))
}

fn exact_subset<T: PartialEq>(old: &[T], new: &[T]) -> bool {
    old.iter().all(|item| new.contains(item))
}

fn by_id(contracts: &[ContractSnapshot]) -> BTreeMap<&str, &ContractSnapshot> {
    contracts
        .iter()
        .map(|contract| (contract.id.as_str(), contract))
        .collect()
}

fn add(
    changes: &mut BTreeSet<CompatibilityChange>,
    category: CompatibilityCategory,
    contract: &str,
) {
    changes.insert(CompatibilityChange {
        category,
        contract: Some(contract.into()),
    });
}
