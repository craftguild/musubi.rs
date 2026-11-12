# ABI compatibility corpus

`input/` contains 25 repo-owned interface samples. Synthetic operation, event,
error, tuple and parameter names keep private application structures out of the
fixture. ABI entry counts, overload groups, tuple nesting, arrays, integer widths,
state mutability and indexed-event positions retain the existing coverage.
The ABI JSON total remains at least 133,404 bytes.

Named ERC-20, Safe 1.4.1, Chainlink and Uniswap interfaces keep their actual
public API names. These are interface samples, not vendored contract
implementations. Required third-party notices elsewhere in this repository still
apply. The private original evaluation archive is not a build or test input.

The closed `input/inventory.json` records actual ABI byte hashes. These authored
inputs make no Solidity compiler, external revision or artifact-coherence claim.
The normal generator records its real producer when building an SDK from them;
never restamp a retained SDK. The separately retained Solidity event, EIP-712 and
Safe source/artifact vectors keep their real compiler provenance and hashes.

Verify from the repository root after a normal CLI build:

```sh
scripts/check-abi-corpus.sh
scripts/test-abi-corpus-manifest.sh
scripts/test-abi-corpus.sh
scripts/test-sdk-fixtures.sh
```

The corpus checker retains closed JSON, complete inventory/hash, no-follow
snapshot, replacement/growth/FIFO and resource-boundary checks. The SDK gates
parse, generate, exactly check and compile all 25 interfaces. No other product
checkout, application backend helper or external service is needed.
