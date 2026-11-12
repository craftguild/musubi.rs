# musubi.rs documentation

Start with the [project README](../README.md) for a source build and manifest
initialization. Runtime and source-build requirements are separate.

## Guides

- [Development and validation](guides/development.md): fixed tools, ordinary
  gates, actual CLI acceptance and recorded SDK replay.

## Design

- [Architecture](design/architecture.md): manifests, inputs, owned SDK APIs,
  retained-handle I/O, recovery, compatibility, runtime and Safe boundaries.
- [Design reading order](design/README.md): current authority and the classification
  of accepted and superseded decisions.

## Architecture decisions

[Numbered ADRs](adr/README.md) retain their original numbers, statuses and
supersession links. Read the architecture and relevant decisions before changing
product contracts. [AGENTS.md](../AGENTS.md) governs implementation and tests.

Beads owns current task and review evidence. Historical pass reports and accepted
decisions do not establish qualification of a changed source tree.
