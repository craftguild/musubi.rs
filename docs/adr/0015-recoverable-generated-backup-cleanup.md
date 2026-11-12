# ADR 0015: Recoverable Generated SDK Backup Cleanup

## Status

Accepted. Amends ADR 0014's failure-state wording so prior SDK bytes remain recoverable across every
fallible backup-cleanup step; does not change the consumer CLI, runtime API, or release identity.

## Context

Recursive deletion can remove some old files before reporting a limit or filesystem error.
Keeping the partially deleted backup directory is insufficient to recover the previous SDK.
Creating another directory copy only moves the same recursive-deletion problem to that copy.

## Decision

- After successful installation but before recursive backup cleanup, persist the validated previous
  generated-file snapshot in one exclusive, no-follow, mode-0600 sibling recovery file. Fsync the
  file and its retained parent handle before starting recursive cleanup.
- Recovery preparation failure preserves the old directory backup and reports failure. Recursive
  cleanup failure preserves both the remaining backup and the complete recovery file. Final
  recovery-file deletion is one atomic unlink with no subsequent fallible cleanup; unlink failure
  retains the complete file. Diagnostics name only the generated relative recovery basename.
- These post-installation failures may leave the new SDK live. The guarantee is complete recovery
  of prior file bytes, not automatic rollback after cleanup has begun. Neither the manifest nor
  unrelated outputs are changed, and a cleanup failure is never reported as successful update.
- The sole recovery format is `musubi/recovery-v1\n`, followed by a big-endian u64 file count.
  Each record contains big-endian u32 relative-path byte length, u64 content length, native Unix
  relative-path bytes, and exact file bytes. Existing effective SDK file/content/depth bounds still
  apply; archive metadata is bounded by 256 bytes per path component plus 12 bytes per record.
  Recovery records cannot contain traversal, controls, backslashes, or non-normal components.
- The recovery contract preserves every file path and byte included by exact SDK inspection.
  It is not a filesystem metadata, permission, empty-directory, compiler-coherence, or authenticity
  attestation. External adversarial deletion of retained recovery evidence is outside this boundary.
- `python3 scripts/extract-sdk-recovery.py RECOVERY NEW-DIRECTORY` validates and extracts only this
  exact format. It never replaces a configured or existing SDK. Operators review the extracted tree
  before deliberately moving live outputs. The recovery file is retained until explicit cleanup
  or a successful generator transaction removes it.
- Before installation fails, existing no-replace rollback remains authoritative. Interrupted
  processes may leave their backup/recovery siblings; no automatic next-run scavenger deletes them.

## Verification

Inject recovery writes, recursive cleanup, final unlink, install, and rollback failures. Confirm all
previous generated files can be recovered and unrelated paths remain unchanged. Exercise malformed,
duplicate, oversized, traversal, special-file, existing-destination, and symlink extractor inputs.
