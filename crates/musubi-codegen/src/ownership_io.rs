//! Public bounded ownership inspection delegates all Unix I/O to the codegen capability reader.

use std::path::Path;

use crate::{GenerateError, GeneratedTree, HardLimits};

/// Retains and validates one exact generated tree without following any path component's symlink.
///
/// No Cargo process or filesystem mutation occurs. Private compilation artifacts must be stored
/// outside this tree. Callers with known system path aliases must explicitly resolve those aliases
/// before supplying the root; this function never silently canonicalizes an untrusted path.
///
/// # Errors
/// Rejects unsupported hosts, malformed ownership, file drift, empty/extra directories, symlinks,
/// special entries, concurrent mutations observed during reads, and effective/hard resource limits.
pub fn read_owned_tree(root: &Path, limits: HardLimits) -> Result<GeneratedTree, GenerateError> {
    crate::host_capability_preflight()
        .map_err(|_| crate::output::io_error(crate::UNSUPPORTED_HOST_MESSAGE))?;
    #[cfg(unix)]
    {
        let files =
            crate::output_fs::read_tree_from_handle(crate::output_read::root(root)?, limits)?;
        GeneratedTree::from_retained(
            files
                .into_iter()
                .map(|(path, contents)| crate::GeneratedFile { path, contents }),
            limits,
        )
        .map_err(GenerateError::from)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, limits);
        Err(crate::output::io_error(crate::UNSUPPORTED_HOST_MESSAGE))
    }
}
