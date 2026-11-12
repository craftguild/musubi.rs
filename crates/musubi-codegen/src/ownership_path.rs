//! Normal UTF-8 ownership paths and component-level file/directory collision detection.

use std::collections::BTreeMap;

use crate::{HardLimits, OwnershipError};

/// Enforces the existing 255-byte component and effective directory-depth bounds.
pub(crate) fn validate(path: &str, limits: HardLimits) -> Result<(), OwnershipError> {
    if path.is_empty() || path.contains('\\') || path.chars().any(char::is_control) {
        return Err(OwnershipError::InvalidPath);
    }
    let root = caseless::default_case_fold_str(path.split('/').next().unwrap_or_default());
    if matches!(root.as_str(), "cargo.lock" | "target" | ".cargo" | "cache") {
        return Err(OwnershipError::InvalidPath);
    }
    let mut depth = 0;
    for part in path.split('/') {
        if matches!(part, "" | "." | "..") || part.len() > 255 {
            return Err(OwnershipError::InvalidPath);
        }
        depth += 1;
    }
    if depth > limits.nesting.saturating_add(1) {
        return Err(OwnershipError::LimitExceeded);
    }
    Ok(())
}

/// Tracks ancestors as well as leaves, so `A/x` versus `a/y` cannot alias on the filesystem.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Paths {
    entries: BTreeMap<String, (String, bool)>,
    directory_count: usize,
}

impl Paths {
    pub(crate) fn insert(
        &mut self,
        path: &str,
        directory: bool,
        maximum: usize,
    ) -> Result<(), OwnershipError> {
        for (index, _) in path.match_indices('/') {
            self.entry(&path[..index], true, maximum)?;
        }
        self.entry(path, directory, maximum)
    }

    fn entry(&mut self, path: &str, directory: bool, maximum: usize) -> Result<(), OwnershipError> {
        let folded = caseless::default_case_fold_str(path);
        if let Some((previous, was_directory)) = self.entries.get(&folded) {
            if previous != path || !directory || !was_directory {
                return Err(OwnershipError::InvalidPath);
            }
        } else {
            if directory {
                if self.directory_count >= maximum {
                    return Err(OwnershipError::LimitExceeded);
                }
                self.directory_count += 1;
            }
            self.entries.insert(folded, (path.into(), directory));
        }
        Ok(())
    }

    pub(crate) fn directories(&self) -> impl Iterator<Item = &str> {
        self.entries
            .values()
            .filter_map(|(path, directory)| directory.then_some(path.as_str()))
    }
}
