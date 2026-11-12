# ADR 0010: Consumer-Resolvable Support Dependencies

## Status

Accepted for compatible support dependencies and consumer-owned lockfiles. ADR 0016 supersedes the
fresh/current release-build rationale and private-SSH-only runtime requirements below. Explicit
manifest-owned Git/Path sources are supported; automatic dependency fallback remains forbidden.

## Context

The reviewed runtime revision used exact patch requirements for Serde and URL. Cargo unifies
semver-compatible versions. An exact `serde = 1.0.219` therefore cannot compose with a consumer
whose existing Alloy dependency requires `serde >= 1.0.226`; it fails before compiling generated
code. Editing generated manifests or downgrading the consumer's security-sensitive binding stack
would violate the generated ownership and consumer-policy boundaries.

Release reproducibility already comes from the reviewed Git revision, tracked `Cargo.lock`, locked
fresh/current builds, and byte comparison. Exact patch requirements in reusable library manifests
do not strengthen those gates, because the consuming repository owns its final lockfile.

## Decision

- Reusable musubi.rs libraries declare compatible minimum-version ranges for general support crates
  such as Serde, serde_json, URL, and ruint.
- The repository lockfile remains the exact graph used by validation and release packaging.
- Generated crates continue to pin `musubi-core` and `musubi-runtime` to one reviewed Git
  revision. This decision does not loosen generator/runtime identity.
- A downstream resolution fixture combines core/runtime with Alloy consensus 2.3.0 and the newer
  Serde, URL and ruint versions. Full validation resolves and compiles the
  repo-owned fixture with its tracked lockfile.
- A dependency-resolution failure remains fail closed. Generation must not insert a path,
  alternate transport, or manifest override as a fallback.

## Consequences

Consumer repositories can resolve one supported semver-compatible support dependency version
while retaining their own reviewed lockfile. Package release remains byte deterministic because
fresh and current release binaries still use the repository locked graph.

Changing a support dependency outside its declared compatibility range still requires an explicit
dependency review and lockfile update.

## Rejected Alternatives

### Edit the generated Cargo manifest

Generated output is owned by codegen and exact drift checking; consumer edits would destroy that
evidence.

### Downgrade the consumer's Alloy stack

Applications own their interaction and numeric policies, and unrelated dependency downgrades are not an
SDK integration mechanism.

### Permit multiple sources or development-path fallback

Fallback would weaken the immutable private-SSH release identity and fresh-clone contract.
