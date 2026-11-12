//! Handle-relative, root-confined, size-bounded filesystem access for inputs.

use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::{ffi::OsStr, fs::File, io::Read as _, path::Component, sync::Arc};

#[cfg(unix)]
use rustix::fs::{self, FileType, Mode, OFlags};

#[cfg(unix)]
use crate::input::failure_at;
use crate::{
    Diagnostic, DiagnosticCode, HardLimits,
    input::{InputError, failure},
};

/// Open manifest-directory capability retained for every referenced input.
#[derive(Clone)]
pub(crate) struct InputRoot {
    #[cfg(unix)]
    descriptor: Arc<std::os::fd::OwnedFd>,
    path: PathBuf,
}

impl InputRoot {
    #[cfg(test)]
    pub(crate) fn for_test(path: &Path) -> Self {
        #[cfg(unix)]
        {
            Self::open(path).unwrap()
        }
        #[cfg(not(unix))]
        {
            Self {
                path: path.to_path_buf(),
            }
        }
    }

    #[cfg(unix)]
    pub(crate) fn open(path: &Path) -> Result<Self, InputError> {
        #[cfg(unix)]
        {
            let path = if path.as_os_str().is_empty() {
                Path::new(".")
            } else {
                path
            };
            let descriptor = open_root(path)?;
            let path = std::env::current_dir()
                .map_err(|_| manifest_unsafe())?
                .join(path);
            Ok(Self {
                descriptor: Arc::new(descriptor),
                path,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(unsupported_error())
        }
    }

    #[cfg(unix)]
    pub(crate) fn descriptor(&self) -> &std::os::fd::OwnedFd {
        &self.descriptor
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Debug for InputRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InputRoot { .. }")
    }
}

/// Opens and reads the manifest without reopening its pathname.
pub(crate) fn open_manifest(path: &Path) -> Result<(InputRoot, Vec<u8>), InputError> {
    #[cfg(unix)]
    {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let name = path.file_name().ok_or_else(manifest_unsafe)?;
        let root = InputRoot::open(parent)?;
        let manifest = open_entry(root.descriptor(), name, false).map_err(|_| manifest_unsafe())?;
        require_regular(&manifest, "manifest is not a regular file", None)?;
        let bytes = read_bounded(
            File::from(manifest),
            HardLimits::V1.manifest_bytes,
            "manifest exceeds the 1 MiB hard limit",
            None,
        )?;
        Ok((root, bytes))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(unsupported_error())
    }
}

pub(crate) fn load_optional(
    root: &InputRoot,
    path: Option<&str>,
    maximum: usize,
    total: &mut usize,
    limits: HardLimits,
) -> Result<Option<Vec<u8>>, InputError> {
    path.map(|path| read_relative(root, path, maximum, total, limits))
        .transpose()
}

pub(crate) fn read_relative(
    root: &InputRoot,
    relative: &str,
    maximum: usize,
    total: &mut usize,
    limits: HardLimits,
) -> Result<Vec<u8>, InputError> {
    #[cfg(unix)]
    {
        read_relative_with_hook(root, relative, maximum, total, limits, || {})
    }
    #[cfg(not(unix))]
    {
        let _ = (root, relative, maximum, total, limits);
        Err(unsupported_error())
    }
}

#[cfg(unix)]
fn read_relative_with_hook(
    root: &InputRoot,
    relative: &str,
    maximum: usize,
    total: &mut usize,
    limits: HardLimits,
    after_open: impl FnOnce(),
) -> Result<Vec<u8>, InputError> {
    if !canonical_relative_path(relative) {
        return Err(path_unsafe(relative));
    }
    let path = Path::new(relative);
    let mut components = path.components().peekable();
    if components.peek().is_none() {
        return Err(path_unsafe(relative));
    }
    let mut directory = None;
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            return Err(path_unsafe(relative));
        };
        let final_component = components.peek().is_none();
        let parent = directory.as_ref().unwrap_or(root.descriptor());
        let descriptor =
            open_entry(parent, name, !final_component).map_err(|_| path_unsafe(relative))?;
        if final_component {
            require_regular(
                &descriptor,
                "input path must name a regular file",
                Some(relative),
            )?;
            let remaining = limits
                .aggregate_input_bytes
                .checked_sub(*total)
                .ok_or_else(|| limit_error(relative))?;
            let effective = maximum.min(remaining);
            reject_large_metadata(&descriptor, effective, relative)?;
            after_open();
            let bytes = read_bounded(
                File::from(descriptor),
                effective,
                "input exceeds its effective resource limit",
                Some(relative),
            )?;
            *total = total
                .checked_add(bytes.len())
                .ok_or_else(|| limit_error(relative))?;
            return Ok(bytes);
        }
        directory = Some(descriptor);
    }
    Err(path_unsafe(relative))
}

#[cfg(unix)]
fn canonical_relative_path(relative: &str) -> bool {
    !relative.is_empty()
        && !relative.starts_with('/')
        && !relative.ends_with('/')
        && !relative.contains('\\')
        && !relative.chars().any(char::is_control)
        && relative
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

#[cfg(unix)]
fn open_root(path: &Path) -> Result<std::os::fd::OwnedFd, InputError> {
    fs::open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| {
        failure(Diagnostic::error(
            DiagnosticCode::UnsafePath,
            "manifest root must be an accessible real directory",
        ))
    })
}

#[cfg(unix)]
fn open_entry<Fd: std::os::fd::AsFd>(
    directory: Fd,
    name: &OsStr,
    directory_only: bool,
) -> Result<std::os::fd::OwnedFd, rustix::io::Errno> {
    let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    if directory_only {
        flags |= OFlags::DIRECTORY;
    }
    fs::openat(directory, name, flags, Mode::empty())
}

#[cfg(unix)]
fn require_regular<Fd: std::os::fd::AsFd>(
    descriptor: Fd,
    message: &str,
    location: Option<&str>,
) -> Result<(), InputError> {
    let metadata = fs::fstat(descriptor).map_err(|_| unreadable(location))?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
        return Err(match location {
            Some(location) => failure_at(DiagnosticCode::UnsafePath, message, location),
            None => failure(Diagnostic::error(DiagnosticCode::UnsafePath, message)),
        });
    }
    Ok(())
}

#[cfg(unix)]
fn reject_large_metadata<Fd: std::os::fd::AsFd>(
    descriptor: Fd,
    maximum: usize,
    relative: &str,
) -> Result<(), InputError> {
    let metadata = fs::fstat(descriptor).map_err(|_| unreadable(Some(relative)))?;
    let size = usize::try_from(metadata.st_size).map_err(|_| limit_error(relative))?;
    if size > maximum {
        return Err(limit_error(relative));
    }
    Ok(())
}

#[cfg(unix)]
fn read_bounded(
    mut file: File,
    maximum: usize,
    message: &str,
    location: Option<&str>,
) -> Result<Vec<u8>, InputError> {
    let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    file.by_ref()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| unreadable(location))?;
    if bytes.len() > maximum {
        return Err(match location {
            Some(location) => failure_at(DiagnosticCode::LimitExceeded, message, location),
            None => failure(Diagnostic::error(DiagnosticCode::LimitExceeded, message)),
        });
    }
    Ok(bytes)
}

#[cfg(unix)]
fn path_unsafe(_relative: &str) -> InputError {
    // A rejected path can itself be a secret or source body, including its apparent basename.
    failure_at(
        DiagnosticCode::UnsafePath,
        "input path must remain below the manifest root without links",
        "input",
    )
}

#[cfg(unix)]
fn manifest_unsafe() -> InputError {
    failure(Diagnostic::error(
        DiagnosticCode::UnsafePath,
        "manifest must be an accessible regular file",
    ))
}

#[cfg(unix)]
fn unreadable(location: Option<&str>) -> InputError {
    match location {
        Some(location) => failure_at(
            DiagnosticCode::InvalidInput,
            "input file cannot be read",
            location,
        ),
        None => failure(Diagnostic::error(
            DiagnosticCode::InvalidInput,
            "manifest cannot be read",
        )),
    }
}

#[cfg(unix)]
fn limit_error(relative: &str) -> InputError {
    failure_at(
        DiagnosticCode::LimitExceeded,
        "input exceeds its effective resource limit",
        relative,
    )
}

#[cfg(not(unix))]
fn unsupported_error() -> InputError {
    failure(Diagnostic::error(
        DiagnosticCode::UnsafePath,
        "secure compiler-input loading is unavailable on this platform",
    ))
}

#[cfg(all(test, unix))]
#[path = "input_fs_tests.rs"]
mod tests;
