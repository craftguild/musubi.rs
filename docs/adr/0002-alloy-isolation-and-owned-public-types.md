# ADR 0002: Alloy isolation and owned public types

## Status

Accepted.

## Context

Exposing Alloy primitives or macro-generated types would tie SDK public semver to
Alloy and force consumer changes when replacing a codec/provider implementation.

## Decision

Own address/integer/bytes/hash, request/receipt and artifact types. Generated
signatures, fields, bounds and rustdoc do not expose Alloy. Do not use `sol!`, its
Solidity parser or macro codegen in the generator. Runtime may privately use
Alloy dynamic ABI; `musubi-alloy` owns provider/wallet/network adaptation.

An object-safe boxed-future SPI separates ReadBackend (chain ID/call/estimate/code/
storage/logs/receipt) and TransactionExecutor (fee/nonce/signing/broadcast/pending
policy). Offline write descriptors are always available; send requires an
executor. Generated contracts do not own keys. Explicit target addresses, matching
read/write mutability and payable-only native value are enforced before execution.
CREATE deployment rejects a request carrying a target address. Generated crates
check the runtime's generated ABI version.

## Consequences and rejected alternatives

Consumers can use owned values and descriptors without Alloy, and adapter updates
remain bounded. Conversion code and golden tests are required; private Alloy
codec use still contributes dependencies. Direct Alloy trait implementations on
SDK types would couple semver. Reimplementing every codec and transaction layer
at once would broaden verification needlessly, so establish the owned boundary
first and retain replaceable private implementations.
