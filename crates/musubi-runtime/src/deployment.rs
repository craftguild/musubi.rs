//! Exact library linking, constructor init code, and runtime template verification.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{
    AbiField, AbiValue, Address, Bytes, CreationBytecode, ImmutableReference, LinkReference,
    Receipt, RuntimeBytecode, TransactionRequest,
};

use crate::{BackendError, CodecError, TransactionExecutor, encode_fields};

/// Exact source/library address mapping used for bytecode linking.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibraryAddresses(BTreeMap<(String, String), Address>);

impl LibraryAddresses {
    /// Creates an empty mapping.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Adds a source-qualified library address.
    pub fn insert(
        &mut self,
        source: impl Into<String>,
        library: impl Into<String>,
        address: Address,
    ) -> Option<Address> {
        self.0.insert((source.into(), library.into()), address)
    }

    fn get(&self, reference: &LinkReference) -> Option<Address> {
        self.0
            .get(&(reference.source.clone(), reference.library.clone()))
            .copied()
    }
}

/// Deployment and runtime-template failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DeploymentError {
    /// One required library is missing or one unexpected address was supplied.
    #[error("library address coverage is not exact")]
    LibraryCoverage,
    /// A bytecode reference layout is malformed, out of bounds, or overlapping.
    #[error("bytecode reference layout is invalid")]
    Reference,
    /// Constructor arguments do not match their ABI descriptor.
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// A deployment request was not a contract-creation transaction.
    #[error("deployment transaction must not specify a target address")]
    TransactionKind,
    /// Transaction policy, signing, broadcast, or receipt completion failed.
    #[error(transparent)]
    Backend(#[from] BackendError),
}

/// Links creation bytecode after enforcing exact source-qualified address coverage.
///
/// # Errors
///
/// Returns [`DeploymentError`] when a required address is missing, an extra address is supplied,
/// or a link range is malformed.
pub fn link_creation(
    bytecode: &CreationBytecode,
    libraries: &LibraryAddresses,
) -> Result<Bytes, DeploymentError> {
    bytecode
        .validate()
        .map_err(|_| DeploymentError::Reference)?;
    link(bytecode.template.as_ref(), &bytecode.links, libraries)
}

/// Builds constructor init code from linked creation bytecode and canonical ABI arguments.
///
/// # Errors
///
/// Returns [`DeploymentError`] for linking or constructor argument failure.
pub fn build_init_code(
    bytecode: &CreationBytecode,
    libraries: &LibraryAddresses,
    fields: &[AbiField],
    values: &[AbiValue],
) -> Result<Bytes, DeploymentError> {
    let linked = link_creation(bytecode, libraries)?;
    let arguments = encode_fields(fields, values)?;
    let mut init_code = Vec::with_capacity(linked.len() + arguments.len());
    init_code.extend_from_slice(linked.as_ref());
    init_code.extend_from_slice(arguments.as_ref());
    Ok(Bytes::new(init_code))
}

/// Executes an already built contract-creation transaction using executor-owned policy.
///
/// # Errors
///
/// Returns [`DeploymentError`] if the request is not contract creation or backend processing
/// fails.
pub async fn execute_deployment<B: TransactionExecutor>(
    backend: &B,
    request: TransactionRequest,
) -> Result<Receipt, DeploymentError> {
    validate_deployment_request(&request)?;
    let filled = backend.fill_transaction(request).await?;
    let transaction = backend.sign_and_broadcast(filled).await?;
    backend
        .wait_for_receipt(transaction)
        .await
        .map_err(Into::into)
}

pub(crate) fn validate_deployment_request(
    request: &TransactionRequest,
) -> Result<(), DeploymentError> {
    if request.to.is_some() {
        Err(DeploymentError::TransactionKind)
    } else {
        Ok(())
    }
}

/// Result of comparing deployed code with a linked, immutable-masked runtime template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeVerification {
    /// Whether byte lengths and every unmasked byte match.
    pub matches: bool,
    /// First unmasked mismatch offset, if any.
    pub first_mismatch: Option<usize>,
}

/// Verifies deployed runtime bytes after linking libraries and masking compiler-declared
/// immutables.
///
/// # Errors
///
/// Returns [`DeploymentError`] if library coverage or a stored reference is invalid.
pub fn verify_runtime(
    runtime: &RuntimeBytecode,
    libraries: &LibraryAddresses,
    deployed: &[u8],
) -> Result<RuntimeVerification, DeploymentError> {
    runtime.validate().map_err(|_| DeploymentError::Reference)?;
    let expected = link(runtime.template.as_ref(), &runtime.links, libraries)?;
    if expected.len() != deployed.len() {
        return Ok(RuntimeVerification {
            matches: false,
            first_mismatch: Some(expected.len().min(deployed.len())),
        });
    }
    let masked = immutable_offsets(&runtime.immutables, expected.len())?;
    let first_mismatch =
        expected
            .as_ref()
            .iter()
            .zip(deployed)
            .enumerate()
            .find_map(|(index, (left, right))| {
                (!masked.contains(&index) && left != right).then_some(index)
            });
    Ok(RuntimeVerification {
        matches: first_mismatch.is_none(),
        first_mismatch,
    })
}

fn link(
    template: &[u8],
    references: &[LinkReference],
    libraries: &LibraryAddresses,
) -> Result<Bytes, DeploymentError> {
    let required: BTreeSet<_> = references
        .iter()
        .map(|reference| (reference.source.clone(), reference.library.clone()))
        .collect();
    if required.len() != libraries.0.len()
        || libraries.0.keys().any(|name| !required.contains(name))
    {
        return Err(DeploymentError::LibraryCoverage);
    }
    let mut linked = template.to_vec();
    for reference in references {
        let address = libraries
            .get(reference)
            .ok_or(DeploymentError::LibraryCoverage)?;
        for range in &reference.ranges {
            if range.length != 20 {
                return Err(DeploymentError::Reference);
            }
            let end = range
                .start
                .checked_add(range.length)
                .ok_or(DeploymentError::Reference)?;
            let slot = linked
                .get_mut(range.start..end)
                .ok_or(DeploymentError::Reference)?;
            slot.copy_from_slice(address.as_ref());
        }
    }
    Ok(Bytes::new(linked))
}

fn immutable_offsets(
    references: &[ImmutableReference],
    length: usize,
) -> Result<BTreeSet<usize>, DeploymentError> {
    let mut offsets = BTreeSet::new();
    for reference in references {
        for range in &reference.ranges {
            let end = range
                .start
                .checked_add(range.length)
                .ok_or(DeploymentError::Reference)?;
            if range.length == 0 || end > length {
                return Err(DeploymentError::Reference);
            }
            offsets.extend(range.start..end);
        }
    }
    Ok(offsets)
}
