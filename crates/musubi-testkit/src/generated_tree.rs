//! Retained generated-crate snapshots use codegen's sole bounded ownership reader.

use crate::GeneratedCrateError;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(crate) struct TreeSnapshot {
    pub(crate) files: BTreeMap<PathBuf, Vec<u8>>,
    pub(crate) bytes: usize,
}

impl TreeSnapshot {
    pub(crate) fn read(root: &Path) -> Result<Self, GeneratedCrateError> {
        let tree = musubi_codegen::read_owned_tree(root, musubi_codegen::HardLimits::V1).map_err(
            |error| {
                if error.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == musubi_codegen::DiagnosticCode::LimitExceeded
                }) {
                    GeneratedCrateError::LimitExceeded
                } else {
                    GeneratedCrateError::InvalidLayout
                }
            },
        )?;
        let bytes = tree.file_entries().map(|(_, bytes)| bytes.len()).sum();
        let files = tree
            .into_files()
            .map(|file| (file.path, file.contents))
            .collect();
        Ok(Self { files, bytes })
    }

    pub(crate) fn file(&self, path: &str) -> Result<&[u8], GeneratedCrateError> {
        self.files
            .get(Path::new(path))
            .map(Vec::as_slice)
            .ok_or(GeneratedCrateError::InvalidLayout)
    }
}
