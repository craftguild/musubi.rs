# ADR 0006: Fail-closed release identity and explicit development generation

## Status

Superseded by [ADR0008](0008-private-ssh-release-identity.md), later superseded
by ADR0016. This is a historical decision, not a current runtime selection mode.

## Context and original decision

Before a real publication URL/revision existed, placeholder release identities
would mislead reviewers. The original design split a closed development/release
runtime union. Development required explicit UTF-8 relative Cargo paths, rejected
absolute paths and recorded distribution=development without a Git revision.
Release required explicit HTTPS and a full lowercase40 nonzero revision, rejecting
empty/placeholder/example URLs, branches and tags. The release gate failed closed
until real reviewed inputs existed; there was no implicit promotion.

## Consequences and rejected alternatives

Local fixtures could compile without pretending to be released. Path-based output
was placement-dependent; publication required separately reviewed source identity.
Fabricated URLs/zero hashes and silently skipping revision validation were both
rejected. ADR0016 now uses explicit manifest Git/Path sources independently of
captured generator observations; do not restore this former distribution union.
