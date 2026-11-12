//! Bounded construction of canonical typed-data root evidence.

use musubi_core::{TypedDataRoot, TypedDataStruct, keccak256};

use crate::{IrError, TypedDataRootEvidence, typed_data_schema::failure};

pub(crate) fn build(
    types: &[TypedDataStruct],
    roots: &[TypedDataRoot],
    maximum_bytes: usize,
) -> Result<(Vec<TypedDataRootEvidence>, usize), IrError> {
    let graph = musubi_runtime::TypedDataGraph::new(types)
        .map_err(|_| failure("typed-data graph is invalid"))?;
    let mut bytes = 0_usize;
    let mut evidence = Vec::with_capacity(roots.len());
    for root in roots {
        let primary = root.primary_type();
        charge(&mut bytes, primary.len(), maximum_bytes)?;
        charge(&mut bytes, 32, maximum_bytes)?;
        let type_string = graph
            .type_string_with_limit(primary, maximum_bytes.saturating_sub(bytes))
            .map_err(|_| failure("typed-data graph is invalid"))?;
        charge(&mut bytes, type_string.len(), maximum_bytes)?;
        let type_hash = keccak256(type_string.as_bytes());
        let (domain_type_string, domain_type_hash) = match root.domain_type() {
            Some(domain) => {
                charge(&mut bytes, domain.len(), maximum_bytes)?;
                charge(&mut bytes, 32, maximum_bytes)?;
                let value = graph
                    .type_string_with_limit(domain, maximum_bytes.saturating_sub(bytes))
                    .map_err(|_| failure("typed-data domain graph is invalid"))?;
                charge(&mut bytes, value.len(), maximum_bytes)?;
                let hash = keccak256(value.as_bytes());
                (Some(value), Some(hash))
            }
            None => (None, None),
        };
        evidence.push(TypedDataRootEvidence {
            primary_type: primary.into(),
            type_string,
            type_hash,
            domain_type: root.domain_type().map(str::to_owned),
            domain_type_string,
            domain_type_hash,
        });
    }
    Ok((evidence, bytes))
}

fn charge(bytes: &mut usize, added: usize, maximum_bytes: usize) -> Result<(), IrError> {
    *bytes = bytes
        .checked_add(added)
        .ok_or_else(|| failure("typed-data root evidence size overflowed"))?;
    if *bytes > maximum_bytes {
        Err(failure(
            "typed-data root evidence exceeds the effective generated-byte limit",
        ))
    } else {
        Ok(())
    }
}
