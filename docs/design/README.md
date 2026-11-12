# musubi.rs design index

Read [Architecture](architecture.md), then the applicable Accepted ADRs. Resolve
contradictions before changing behavior. Current adoption and review tracking use
Beads; prior plans, prompts and review/evaluation originals are
hash-preserved in the ignored private archive and are historical evidence.
Acceptance of a decision, an old passing result or a configured initializer does
not establish current product qualification.

## Accepted ADRs

- [ADR 0020: Shared build execution and Craftguild standards](../adr/0020-craftguild-standards-adoption.md)
- [ADR 0019: Make build and local CLI installation](../adr/0019-make-build-and-local-install.md)
- [ADR 0018: Musubi product, CLI and package identity](../adr/0018-musubi-product-and-package-identity.md)
- [ADR 0017: Semantic generated layout and ownership-based size exemption](../adr/0017-semantic-generated-layout-and-owned-line-budget.md)
- [ADR 0016: Lightweight validation and generator build identity](../adr/0016-lightweight-validation-and-generator-build-identity.md)
- [ADR 0015: Recoverable Generated SDK Backup Cleanup](../adr/0015-recoverable-generated-backup-cleanup.md)
- [ADR 0014: Manifest-Only Consumer CLI And Publisher-Owned Release Boundary](../adr/0014-manifest-only-cli-and-release-boundary.md)
- [ADR 0013: Handle-Bound Artifact I/O And Conformance Snapshots](../adr/0013-handle-bound-artifact-io.md)
- [ADR 0012: Closed Typed-Data Authority](../adr/0012-closed-typed-data-authority.md)
- [ADR 0011: Explicit Deployable And Interface Artifact Kinds](../adr/0011-explicit-deployable-and-interface-artifact-kinds.md)
- [ADR 0010: Consumer-Resolvable Support Dependencies](../adr/0010-consumer-resolvable-support-dependencies.md)
- [ADR 0009: Separate wire shape and owned tuple surface identity](../adr/0009-separate-wire-and-owned-tuple-identity.md)
- [ADR 0005: Development and validation lifecycle](../adr/0005-development-and-validation-lifecycle.md)
- [ADR 0004: Versioned Safe adapter](../adr/0004-versioned-safe-adapter.md)
- [ADR 0003: Deterministic generated SDK and release identity](../adr/0003-deterministic-generated-sdk-and-release-identity.md)
- [ADR 0002: Alloy isolation and owned public types](../adr/0002-alloy-isolation-and-owned-public-types.md)
- [ADR 0001: External compiler artifact boundary](../adr/0001-external-compiler-artifact-boundary.md)

## Superseded decisions

- [ADR 0006: Fail-closed release identity and explicit development generation](../adr/0006-fail-closed-release-identity.md) was superseded by
  [ADR 0008: Private SSH Release Identity](../adr/0008-private-ssh-release-identity.md), then ADR0016.
- [ADR 0007: Secure host operation boundary](../adr/0007-secure-host-operation-boundary.md) was superseded by ADR0013.
- ADR0020 supersedes local Make/build/measurement portions of earlier decisions.
  Their product protocol, retained-handle I/O, atomic recovery, ABI and generated
  ownership contracts remain in force unless explicitly amended.

## Current boundaries

Config-v4 owns generation and explicit Git/Path runtime source. ABI alone defines
wire semantics, with verified optional AST enrichment and an independent closed
EIP-712 authority. Generated public APIs use owned types; Safe support is reviewed
1.4.1 only. Closed generated-v2 inventories and canonical recorded-producer replay
are both required. Linux/macOS are supported; Windows and live consumer cutovers
are outside this adoption. Shared build/release declarations are distinct from
actual publication, service operations and independent review.
