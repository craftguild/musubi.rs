//! Safe 1.4.1 signature variants and canonical packed layout validation.

use musubi_core::{Address, Bytes, Hash};

use crate::hash::address_word;

/// Maximum accepted packed signature blob, including dynamic EIP-1271 tails.
pub const MAX_SAFE_SIGNATURE_BYTES: usize = 16 * 1024 * 1024;
const STATIC_SIGNATURE_BYTES: usize = 65;

/// Canonical ECDSA recovery identifier accepted by Safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EoaRecoveryId {
    /// Recovery identifier encoded as `27`.
    V27,
    /// Recovery identifier encoded as `28`.
    V28,
}

impl EoaRecoveryId {
    const fn eip712_v(self) -> u8 {
        match self {
            Self::V27 => 27,
            Self::V28 => 28,
        }
    }
}

/// Signature forms implemented by Safe 1.4.1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafeV141Signature {
    /// ECDSA signature directly over the Safe EIP-712 digest.
    Eip712 {
        /// Owner expected to recover from this signature.
        owner: Address,
        /// ECDSA `r` scalar.
        r: Hash,
        /// ECDSA `s` scalar.
        s: Hash,
        /// Canonical recovery identifier.
        recovery_id: EoaRecoveryId,
    },
    /// ECDSA signature over the Ethereum signed-message form of the Safe digest.
    EthSign {
        /// Owner expected to recover from this signature.
        owner: Address,
        /// ECDSA `r` scalar.
        r: Hash,
        /// ECDSA `s` scalar.
        s: Hash,
        /// Canonical recovery identifier before Safe's `+4` wire marker.
        recovery_id: EoaRecoveryId,
    },
    /// On-chain approval previously recorded by `approveHash`.
    ApprovedHash {
        /// Owner whose approval mapping must contain the transaction hash.
        owner: Address,
    },
    /// Dynamic EIP-1271 signature checked by an owner contract.
    Contract {
        /// Owner contract called for signature validation.
        owner: Address,
        /// Opaque signature bytes passed to the owner contract.
        signature: Bytes,
    },
}

impl SafeV141Signature {
    /// Returns the owner used for deterministic ordering and duplicate rejection.
    #[must_use]
    pub const fn owner(&self) -> Address {
        match self {
            Self::Eip712 { owner, .. }
            | Self::EthSign { owner, .. }
            | Self::ApprovedHash { owner }
            | Self::Contract { owner, .. } => *owner,
        }
    }
}

/// Safe signature blob packed from an explicitly owned, canonical signature list.
///
/// The only public construction path is [`pack_signatures`]. Raw bytes cannot establish EOA owner
/// ordering without a transaction digest, so layout-only parsing is deliberately not exposed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedSignatures {
    bytes: Bytes,
    signature_count: usize,
}

impl PackedSignatures {
    /// Borrows canonical signature bytes for `execTransaction`.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    /// Consumes the wrapper and returns canonical signature bytes.
    #[must_use]
    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }

    /// Returns the number of static 65-byte signature records.
    #[must_use]
    pub const fn signature_count(&self) -> usize {
        self.signature_count
    }
}

/// Fail-closed signature packing or layout error.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SignatureError {
    /// A Safe threshold must contain at least one signature.
    #[error("Safe signature threshold must be non-zero")]
    ZeroThreshold,
    /// The supplied signature count differs from the requested threshold.
    #[error("Safe signature count {actual} does not equal threshold {expected}")]
    ThresholdMismatch {
        /// Required signature count.
        expected: usize,
        /// Supplied signature count.
        actual: usize,
    },
    /// Two signature records claim the same owner.
    #[error("duplicate Safe signature owner {owner}")]
    DuplicateOwner {
        /// Duplicated owner address.
        owner: Address,
    },
    /// Packed signatures exceed the hard memory and parsing limit.
    #[error("Safe signature blob exceeds the hard limit")]
    BlobTooLarge,
    /// Size arithmetic overflowed while validating untrusted layout data.
    #[error("Safe signature layout size overflow")]
    SizeOverflow,
    /// A static signature uses a marker unsupported by Safe 1.4.1.
    #[error("unsupported Safe 1.4.1 signature marker {marker} at index {index}")]
    UnsupportedMarker {
        /// Static signature index.
        index: usize,
        /// Wire `v` marker.
        marker: u8,
    },
    /// A marker-specific static word contains ignored non-canonical bytes.
    #[error("non-canonical Safe static signature record at index {index}")]
    NonCanonicalStaticRecord {
        /// Static signature index.
        index: usize,
    },
    /// A contract signature offset is non-canonical or outside the blob.
    #[error("invalid contract signature offset at index {index}")]
    InvalidContractOffset {
        /// Static signature index.
        index: usize,
    },
    /// A contract signature length is out of bounds.
    #[error("invalid contract signature length at index {index}")]
    InvalidContractLength {
        /// Static signature index.
        index: usize,
    },
    /// Dynamic signature padding contains non-zero bytes.
    #[error("non-canonical contract signature padding at index {index}")]
    NonCanonicalPadding {
        /// Static signature index.
        index: usize,
    },
    /// Bytes remain after the final dynamic signature segment.
    #[error("trailing bytes follow Safe contract signatures")]
    TrailingDynamicData,
}

/// Sorts owner signatures and packs exactly one signature per threshold slot.
///
/// # Errors
///
/// Returns [`SignatureError`] for zero or mismatched threshold, duplicate owners, excessive input,
/// or any impossible dynamic layout.
pub fn pack_signatures(
    mut signatures: Vec<SafeV141Signature>,
    threshold: usize,
) -> Result<PackedSignatures, SignatureError> {
    if threshold == 0 {
        return Err(SignatureError::ZeroThreshold);
    }
    if signatures.len() != threshold {
        return Err(SignatureError::ThresholdMismatch {
            expected: threshold,
            actual: signatures.len(),
        });
    }
    if signatures.len() > MAX_SAFE_SIGNATURE_BYTES / STATIC_SIGNATURE_BYTES {
        return Err(SignatureError::BlobTooLarge);
    }
    signatures.sort_by_key(SafeV141Signature::owner);
    for pair in signatures.windows(2) {
        if pair[0].owner() == pair[1].owner() {
            return Err(SignatureError::DuplicateOwner {
                owner: pair[0].owner(),
            });
        }
    }
    let static_size = signatures
        .len()
        .checked_mul(STATIC_SIGNATURE_BYTES)
        .ok_or(SignatureError::SizeOverflow)?;
    let total_size = signatures.iter().try_fold(static_size, |size, signature| {
        size.checked_add(dynamic_size(signature)?)
            .ok_or(SignatureError::SizeOverflow)
    })?;
    if total_size > MAX_SAFE_SIGNATURE_BYTES {
        return Err(SignatureError::BlobTooLarge);
    }
    let mut static_bytes = Vec::with_capacity(static_size);
    let mut dynamic_bytes = Vec::with_capacity(total_size - static_size);
    for signature in signatures {
        encode_signature(
            &signature,
            static_size,
            &mut static_bytes,
            &mut dynamic_bytes,
        )?;
    }
    static_bytes.extend_from_slice(&dynamic_bytes);
    validate_layout(&static_bytes, threshold)?;
    Ok(PackedSignatures {
        bytes: Bytes::new(static_bytes),
        signature_count: threshold,
    })
}

fn dynamic_size(signature: &SafeV141Signature) -> Result<usize, SignatureError> {
    let SafeV141Signature::Contract { signature, .. } = signature else {
        return Ok(0);
    };
    let padded = signature
        .len()
        .checked_add(31)
        .ok_or(SignatureError::SizeOverflow)?
        / 32
        * 32;
    padded.checked_add(32).ok_or(SignatureError::SizeOverflow)
}

fn encode_signature(
    signature: &SafeV141Signature,
    static_size: usize,
    output: &mut Vec<u8>,
    dynamic: &mut Vec<u8>,
) -> Result<(), SignatureError> {
    match signature {
        SafeV141Signature::Eip712 {
            r, s, recovery_id, ..
        } => {
            append_static(output, r.as_ref(), s.as_ref(), recovery_id.eip712_v());
        }
        SafeV141Signature::EthSign {
            r, s, recovery_id, ..
        } => {
            append_static(output, r.as_ref(), s.as_ref(), recovery_id.eip712_v() + 4);
        }
        SafeV141Signature::ApprovedHash { owner } => {
            append_static(output, &address_word(*owner), &[0; 32], 1);
        }
        SafeV141Signature::Contract { owner, signature } => {
            let offset = static_size
                .checked_add(dynamic.len())
                .ok_or(SignatureError::SizeOverflow)?;
            append_static(output, &address_word(*owner), &usize_word(offset), 0);
            dynamic.extend_from_slice(&usize_word(signature.len()));
            dynamic.extend_from_slice(signature.as_ref());
            let padded = dynamic_size(&SafeV141Signature::Contract {
                owner: *owner,
                signature: signature.clone(),
            })? - 32;
            dynamic.resize(dynamic.len() + padded - signature.len(), 0);
        }
    }
    Ok(())
}

fn append_static(output: &mut Vec<u8>, r: &[u8], s: &[u8], v: u8) {
    output.extend_from_slice(r);
    output.extend_from_slice(s);
    output.push(v);
}

fn usize_word(value: usize) -> [u8; 32] {
    let mut word = [0_u8; 32];
    word[24..].copy_from_slice(&(value as u64).to_be_bytes());
    word
}

fn read_usize_word(word: &[u8]) -> Option<usize> {
    if word.len() != 32 || word[..24].iter().any(|byte| *byte != 0) {
        return None;
    }
    let value = u64::from_be_bytes(word[24..].try_into().ok()?);
    usize::try_from(value).ok()
}

fn validate_layout(bytes: &[u8], signature_count: usize) -> Result<(), SignatureError> {
    if signature_count == 0 {
        return Err(SignatureError::ZeroThreshold);
    }
    if bytes.len() > MAX_SAFE_SIGNATURE_BYTES {
        return Err(SignatureError::BlobTooLarge);
    }
    let static_size = signature_count
        .checked_mul(STATIC_SIGNATURE_BYTES)
        .ok_or(SignatureError::SizeOverflow)?;
    if bytes.len() < static_size {
        return Err(SignatureError::InvalidContractOffset { index: 0 });
    }
    let mut expected_dynamic = static_size;
    for index in 0..signature_count {
        let base = index * STATIC_SIGNATURE_BYTES;
        match bytes[base + 64] {
            0 => {
                validate_address_word(&bytes[base..base + 32], index)?;
                let offset = read_usize_word(&bytes[base + 32..base + 64])
                    .filter(|offset| *offset == expected_dynamic)
                    .ok_or(SignatureError::InvalidContractOffset { index })?;
                let length_end = offset.checked_add(32).ok_or(SignatureError::SizeOverflow)?;
                let length = bytes
                    .get(offset..length_end)
                    .and_then(read_usize_word)
                    .ok_or(SignatureError::InvalidContractLength { index })?;
                let padded = length.checked_add(31).ok_or(SignatureError::SizeOverflow)? / 32 * 32;
                let data_end = length_end
                    .checked_add(length)
                    .ok_or(SignatureError::SizeOverflow)?;
                expected_dynamic = length_end
                    .checked_add(padded)
                    .filter(|end| *end <= bytes.len())
                    .ok_or(SignatureError::InvalidContractLength { index })?;
                if bytes[data_end..expected_dynamic]
                    .iter()
                    .any(|byte| *byte != 0)
                {
                    return Err(SignatureError::NonCanonicalPadding { index });
                }
            }
            1 => {
                validate_address_word(&bytes[base..base + 32], index)?;
                if bytes[base + 32..base + 64].iter().any(|byte| *byte != 0) {
                    return Err(SignatureError::NonCanonicalStaticRecord { index });
                }
            }
            27 | 28 | 31 | 32 => {}
            marker => return Err(SignatureError::UnsupportedMarker { index, marker }),
        }
    }
    if expected_dynamic != bytes.len() {
        return Err(SignatureError::TrailingDynamicData);
    }
    Ok(())
}

fn validate_address_word(word: &[u8], index: usize) -> Result<(), SignatureError> {
    if word[..12].iter().any(|byte| *byte != 0) {
        Err(SignatureError::NonCanonicalStaticRecord { index })
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn validate_packed_signature_layout(
    bytes: &[u8],
    signature_count: usize,
) -> Result<(), SignatureError> {
    validate_layout(bytes, signature_count)
}
