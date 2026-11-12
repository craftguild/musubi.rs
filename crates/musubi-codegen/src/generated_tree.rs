//! Bounded output assembly reserves the exact final ownership-marker cost before accepting bytes.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{
    GENERATED_MARKER, HardLimits, MAX_MARKER_BYTES, RenderError, ownership,
    ownership_path::{self, Paths},
    render::render_error,
};

/// One deterministic generated file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFile {
    /// Normal slash-separated relative output path.
    pub path: PathBuf,
    /// Exact file bytes.
    pub contents: Vec<u8>,
}

/// Canonically ordered generated crate tree with bounded content and closed ownership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedTree {
    files: BTreeMap<PathBuf, Vec<u8>>,
    limits: HardLimits,
    bytes: usize,
    paths: Paths,
    /// Present only while assembling final bytes; includes framing and already inserted records.
    marker_reservation: Option<usize>,
}

impl Default for GeneratedTree {
    fn default() -> Self {
        Self::with_limits(HardLimits::V1)
    }
}

impl GeneratedTree {
    /// Constructs an exact owned tree from retained bytes without formatting or I/O.
    ///
    /// # Errors
    /// Rejects invalid ownership, duplicate/unsafe/colliding paths and excessive files/bytes/depth.
    pub fn from_files(files: impl IntoIterator<Item = GeneratedFile>) -> Result<Self, RenderError> {
        Self::from_retained(files, HardLimits::V1)
    }

    pub(crate) fn from_retained(
        files: impl IntoIterator<Item = GeneratedFile>,
        limits: HardLimits,
    ) -> Result<Self, RenderError> {
        let mut tree = Self::with_limits(ownership::bounded_limits(limits));
        tree.marker_reservation = None;
        for file in files {
            tree.accept(file.path, file.contents)?;
        }
        tree.validate_ownership()?;
        Ok(tree)
    }

    pub(crate) fn with_limits(limits: HardLimits) -> Self {
        let limits = ownership::bounded_limits(limits);
        let mut paths = Paths::default();
        paths
            .insert(GENERATED_MARKER, false, limits.generated_files)
            .expect("fixed marker path");
        Self {
            files: BTreeMap::new(),
            limits,
            bytes: 0,
            paths,
            marker_reservation: Some(ownership::empty_marker_bytes()),
        }
    }

    #[cfg(unix)]
    pub(crate) fn limits(&self) -> HardLimits {
        self.limits
    }

    pub(crate) fn remaining_bytes(&self) -> usize {
        self.limits
            .generated_bytes
            .saturating_sub(self.bytes)
            .saturating_sub(self.marker_reservation.unwrap_or(0))
    }

    /// Iterates over owned copies of files in relative path order.
    pub fn files(&self) -> impl Iterator<Item = GeneratedFile> + '_ {
        self.files.iter().map(|(path, contents)| GeneratedFile {
            path: path.clone(),
            contents: contents.clone(),
        })
    }

    /// Borrows final files without duplicating the generated content buffers.
    pub fn file_entries(&self) -> impl Iterator<Item = (&Path, &[u8])> + '_ {
        self.files
            .iter()
            .map(|(path, contents)| (path.as_path(), contents.as_slice()))
    }

    /// Transfers retained buffers to a caller without cloning the file contents.
    pub fn into_files(self) -> impl Iterator<Item = GeneratedFile> {
        self.files
            .into_iter()
            .map(|(path, contents)| GeneratedFile { path, contents })
    }

    /// Revalidates closed ownership without writing or normalizing any bytes.
    ///
    /// # Errors
    /// Rejects missing/invalid markers and any path, inventory, hash or budget inconsistency.
    pub fn validate_ownership(&self) -> Result<(), RenderError> {
        ownership::validate_owned_files(&self.files, self.limits)
            .map(|_| ())
            .map_err(ownership_error)
    }

    pub(crate) fn insert(
        &mut self,
        path: impl Into<PathBuf>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<(), RenderError> {
        let path = path.into();
        let mut contents = contents.into();
        if path.extension().is_some_and(|extension| extension == "rs") {
            while contents.last().is_some_and(u8::is_ascii_whitespace) {
                contents.pop();
            }
            contents.push(b'\n');
        }
        self.accept(path, contents)
    }

    pub(crate) fn insert_slice(
        &mut self,
        path: impl Into<PathBuf>,
        contents: &[u8],
    ) -> Result<(), RenderError> {
        let path = path.into();
        self.validate_insert(&path, contents.len())?;
        self.accept(path, contents.to_vec())
    }

    fn validate_insert(&self, path: &Path, length: usize) -> Result<Option<usize>, RenderError> {
        let name = path
            .to_str()
            .ok_or_else(|| render_error("generated path is not UTF-8"))?;
        ownership_path::validate(name, self.limits).map_err(ownership_error)?;
        if self.files.contains_key(path) {
            return Err(render_error("generated path is duplicated"));
        }
        let reservation = if name == GENERATED_MARKER {
            if length > MAX_MARKER_BYTES {
                return Err(render_error("ownership marker exceeds its byte budget"));
            }
            None
        } else {
            self.marker_reservation
                .map(|previous| {
                    ownership::record_bytes(name, length).and_then(|record| {
                        previous
                            .checked_add(record + usize::from(!self.files.is_empty()))
                            .ok_or_else(|| render_error("ownership marker size overflowed"))
                    })
                })
                .transpose()?
        };
        let total = self
            .bytes
            .checked_add(length)
            .and_then(|bytes| bytes.checked_add(reservation.unwrap_or(0)))
            .ok_or_else(|| render_error("generated output byte count overflowed"))?;
        let files = self.files.len() + 1 + usize::from(reservation.is_some());
        if files > self.limits.generated_files
            || total > self.limits.generated_bytes
            || reservation.is_some_and(|bytes| bytes > MAX_MARKER_BYTES)
        {
            return Err(render_error("generated output exceeds effective limits"));
        }
        Ok(reservation)
    }

    fn accept(&mut self, path: PathBuf, contents: Vec<u8>) -> Result<(), RenderError> {
        let reservation = self.validate_insert(&path, contents.len())?;
        if path != Path::new(GENERATED_MARKER) {
            self.paths
                .insert(
                    path.to_str().expect("validated path"),
                    false,
                    self.limits.generated_files,
                )
                .map_err(ownership_error)?;
        }
        self.marker_reservation = reservation;
        self.bytes += contents.len();
        self.files.insert(path, contents);
        Ok(())
    }

    /// Called only after all Rust normalization/assets/provenance are final. Test fixtures may
    /// explicitly reseal their own synthetic bytes; production exposes no repair/force operation.
    pub(crate) fn seal(&mut self) -> Result<(), RenderError> {
        if let Some(previous) = self.files.remove(Path::new(GENERATED_MARKER)) {
            self.bytes -= previous.len();
        }
        let marker = ownership::render_marker(
            &self.files,
            self.limits.generated_bytes.saturating_sub(self.bytes),
        )?;
        self.marker_reservation = None;
        self.accept(PathBuf::from(GENERATED_MARKER), marker)?;
        self.validate_ownership()
    }
}

fn ownership_error(error: crate::OwnershipError) -> RenderError {
    let code = if error == crate::OwnershipError::LimitExceeded {
        crate::DiagnosticCode::LimitExceeded
    } else {
        crate::DiagnosticCode::UnsafePath
    };
    RenderError {
        diagnostic: crate::Diagnostic::error(code, error.to_string()),
    }
}
