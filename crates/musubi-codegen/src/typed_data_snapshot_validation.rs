//! Effective-limit semantic validation for typed-data snapshots and public IR.

use std::collections::BTreeSet;

use musubi_core::{TypedDataRoot, TypedDataStruct};

use crate::{HardLimits, IrError, TypedDataRootEvidence, names};

pub(crate) fn validate(
    types: &[TypedDataStruct],
    roots: &[TypedDataRoot],
    evidence: &[TypedDataRootEvidence],
    limits: HardLimits,
    maximum_evidence_bytes: usize,
) -> Result<usize, IrError> {
    let types_sorted = types.windows(2).all(|pair| pair[0].name < pair[1].name);
    let roots_sorted = roots.windows(2).all(|pair| {
        (pair[0].primary_type(), pair[0].domain_type())
            < (pair[1].primary_type(), pair[1].domain_type())
    });
    let components = types
        .iter()
        .try_fold(0_usize, |count, ty| count.checked_add(ty.fields.len()));
    if types.is_empty()
        || roots.is_empty()
        || types.len() > limits.typed_data_types
        || roots.len() > limits.typed_data_types
        || components.is_none_or(|count| count > limits.typed_data_fields)
        || !types_sorted
        || !roots_sorted
    {
        return Err(failure("typed-data snapshot ordering is invalid"));
    }
    let graph = musubi_runtime::TypedDataGraph::new(types)
        .map_err(|_| failure("typed-data snapshot graph is invalid"))?;
    if graph.maximum_nesting() > limits.nesting {
        return Err(failure(
            "typed-data snapshot nesting exceeds the effective limit",
        ));
    }
    if graph.maximum_fixed_array_elements() > limits.typed_data_fixed_array_elements {
        return Err(failure(
            "typed-data snapshot fixed array exceeds the effective limit",
        ));
    }
    let mut rust_types = names::reserved_typed_data_type_names();
    for ty in types {
        crate::typed_data_schema::identifier(&ty.name, limits)?;
        if !rust_types.insert(names::rust_pascal(&ty.name)) {
            return Err(failure("typed-data snapshot type names collide"));
        }
        let mut fields = BTreeSet::new();
        let mut rust_fields = BTreeSet::new();
        for (index, field) in ty.fields.iter().enumerate() {
            crate::typed_data_schema::identifier(&field.name, limits)?;
            if !fields.insert(field.name.as_str())
                || !rust_fields.insert(names::rust_field(&field.name, index))
            {
                return Err(failure("typed-data snapshot field names collide"));
            }
        }
    }
    crate::typed_data_schema::validate_roots_and_reachability(types, roots)?;
    let (expected, evidence_bytes) =
        crate::typed_data_evidence::build(types, roots, maximum_evidence_bytes)?;
    if expected != evidence {
        return Err(failure("typed-data snapshot root evidence is invalid"));
    }
    Ok(evidence_bytes)
}

fn failure(message: &str) -> IrError {
    crate::typed_data_schema::failure(message)
}
