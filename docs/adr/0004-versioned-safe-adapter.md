# ADR 0004: Versioned Safe adapter

## Status

Accepted; amended by ADR0012 for generated ABI and independent typed-data authority.

## Context

Safe versions can differ in ABI, signatures, validation and deployments. One open
signature enum would allow unsupported variants on an older contract. Keep a
reusable adapter distinct from application governance.

## Decision

Use sealed SafeVersionSpec markers and version-specific signature unions. Only
SafeV141 is implemented/covered; new versions require a new marker, artifacts,
vectors and actual integration. Do not add future P-256 variants to V141 or own
Safe Transaction Service behavior here.

V141 supports SafeTx/domain EIP-712 hashing, CALL/DELEGATECALL, approveHash,
execTransaction, EIP-712 EOA, eth_sign EOA, approved-hash and EIP-1271 signatures.
Before sorting, reject static counts exceeding blob limits. Packing enforces
declared owner order, duplicates, threshold, marker0/1 canonical address words,
approved-hash zero s, dynamic offsets/bounds/padding and no trailing data.
PackedSignatures has no raw-layout constructor: without a digest it cannot
recover an EOA owner, and contract signatures require an on-chain validity check.

Calls/events/errors use reviewed generated ABI; SafeTx/domain hashes use closed
schemas and Solidity-linked vectors. Keep signature policy/layout in the adapter.
Reviewed pins remain:

- safe-smart-account: `bf943f80fec5ac647159d26161446ac5d716a294`
- safe-deployments: `5bb0ebd7150a777f39bec4733e4d799c4b637b49`

Require exact compile-time-reviewed singleton/proxy-factory/fallback-handler names,
addresses, runtime code hashes and complete upstream asset SHA-256. Address or
VERSION() alone is no proof. SHA-512-pinned official npm artifacts retain Safe.sol
identical to SHA-256-pinned source from the immutable smart-account commit URL.
Registry-v2 binds source revision/digest; any URL/revision/digest/source mismatch
fails before generation.

The explicit updater finishes acquisition/hash/extraction/SDK checks in one
repository-local same-filesystem staging tree. Verify staging/output/backup device
identity, then replace registry, fixtures, inputs and SDK as one no-replace rename
transaction. On failure roll back every path in reverse order, continue after an
individual rollback error and retain/report every unrestored backup. Progressive
copy into production paths is disallowed. Application owner/threshold/module/guard/
fallback policy remains outside this generic adapter.

## Consequences and rejected alternatives

Offline packing and contract validation are different guarantees. Versions require
explicit new evidence; a cross-version signature enum and an application-specific
2-of-2 policy would each violate this boundary.
