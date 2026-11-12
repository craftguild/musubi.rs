//! Handle-relative, bounded inspection of existing generated output trees.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::{ffi::OsStr, fs::File, os::unix::ffi::OsStrExt as _};

#[cfg(unix)]
use rustix::{
    fs::{self, FileType, Mode, OFlags},
    io::Errno,
};

use crate::{GenerateError, HardLimits, output::unsafe_error};

pub(crate) const MARKER: &str = crate::GENERATED_MARKER;

#[cfg(unix)]
pub(crate) fn validate_existing_at<Fd: std::os::fd::AsFd>(
    parent: Fd,
    name: &std::ffi::OsStr,
    limits: HardLimits,
) -> Result<bool, GenerateError> {
    let Some(files) = read_tree_at(parent, name, limits)? else {
        return Ok(false);
    };
    crate::validate_owned_files(&files, limits).map_err(ownership_error)?;
    Ok(true)
}

#[cfg(unix)]
pub(crate) fn read_tree_at<Fd: std::os::fd::AsFd>(
    parent: Fd,
    name: &std::ffi::OsStr,
    limits: HardLimits,
) -> Result<Option<BTreeMap<PathBuf, Vec<u8>>>, GenerateError> {
    let root = match fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(root) => root,
        Err(error) if error == Errno::NOENT => return Ok(None),
        Err(_) => return Err(unsafe_error("generated target must be a real directory")),
    };
    read_tree_from_handle(root, limits).map(Some)
}

#[cfg(test)]
pub(crate) fn read_tree_with_limits(
    root: &Path,
    limits: HardLimits,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, GenerateError> {
    #[cfg(unix)]
    {
        read_tree_unix(root, limits)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, limits);
        Err(unsafe_error(
            "secure generated-tree inspection is unavailable on this platform",
        ))
    }
}

#[cfg(all(test, unix))]
fn read_tree_unix(
    root: &Path,
    limits: HardLimits,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, GenerateError> {
    read_tree_from_handle(
        crate::output_read::root(
            &root
                .canonicalize()
                .map_err(|_| io_error("test root cannot be resolved"))?,
        )?,
        limits,
    )
}

#[cfg(unix)]
pub(crate) fn read_tree_from_handle(
    root: std::os::fd::OwnedFd,
    limits: HardLimits,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, GenerateError> {
    let limits = crate::ownership::bounded_limits(limits);
    let mut output = BTreeMap::new();
    let mut actual_directories = BTreeSet::new();
    let mut paths = crate::ownership_path::Paths::default();
    let mut stack = vec![crate::output_read::Directory::new(root, PathBuf::new(), 0)?];
    let mut bytes = 0_usize;
    while let Some(frame) = stack.last_mut() {
        let Some(entry) = frame.entries.next() else {
            stack.pop().expect("live traversal frame").finish()?;
            continue;
        };
        let entry = entry.map_err(|_| io_error("generated directory entry cannot be read"))?;
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        if name.is_empty()
            || name.contains(&b'/')
            || name.contains(&b'\\')
            || std::str::from_utf8(name)
                .map_err(|_| unsafe_error("generated entry name is not UTF-8"))?
                .chars()
                .any(char::is_control)
        {
            return Err(unsafe_error(
                "generated directory contains an unsafe entry name",
            ));
        }
        let name = OsStr::from_bytes(name);
        let relative = frame.relative.join(name);
        let text = relative
            .to_str()
            .ok_or_else(|| unsafe_error("generated path is not UTF-8"))?;
        crate::ownership_path::validate(text, limits).map_err(ownership_error)?;
        frame.count += 1;
        let descriptor = crate::output_read::entry(&frame.descriptor, name)?;
        let metadata = fs::fstat(&descriptor)
            .map_err(|_| io_error("generated entry metadata cannot be read"))?;
        match FileType::from_raw_mode(metadata.st_mode) {
            FileType::Directory => {
                paths
                    .insert(text, true, limits.generated_files)
                    .map_err(ownership_error)?;
                actual_directories.insert(relative.clone());
                let depth = frame.depth + 1;
                if depth > limits.nesting {
                    return Err(limit_error(
                        "generated directory depth exceeds effective limits",
                    ));
                }
                stack.push(crate::output_read::Directory::new(
                    descriptor, relative, depth,
                )?);
            }
            FileType::RegularFile => {
                paths
                    .insert(text, false, limits.generated_files)
                    .map_err(ownership_error)?;
                if output.len() >= limits.generated_files {
                    return Err(limit_error("generated file count exceeds effective limits"));
                }
                let contents = read_regular(
                    File::from(descriptor),
                    &metadata,
                    &relative,
                    limits,
                    &mut bytes,
                )?;
                output.insert(relative, contents);
            }
            FileType::Symlink => {
                return Err(unsafe_error("generated directory contains a symlink"));
            }
            _ => {
                return Err(unsafe_error(
                    "generated directory contains a special filesystem entry",
                ));
            }
        }
    }
    let ancestors: BTreeSet<_> = output
        .keys()
        .flat_map(|path| path.ancestors().skip(1))
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .collect();
    if ancestors != actual_directories {
        return Err(unsafe_error(
            "generated directories differ from owned file ancestors",
        ));
    }
    Ok(output)
}

fn read_regular(
    file: File,
    metadata: &rustix::fs::Stat,
    path: &Path,
    limits: HardLimits,
    bytes: &mut usize,
) -> Result<Vec<u8>, GenerateError> {
    let remaining = limits
        .generated_bytes
        .checked_sub(*bytes)
        .ok_or_else(|| limit_error("generated output bytes exceed effective limits"))?;
    let maximum = if path == Path::new(MARKER) {
        remaining.min(crate::MAX_MARKER_BYTES)
    } else {
        remaining
    };
    let contents = crate::output_read::bounded(file, metadata, maximum)?;
    *bytes += contents.len();
    Ok(contents)
}

pub(crate) fn ownership_error(error: crate::OwnershipError) -> GenerateError {
    if error == crate::OwnershipError::LimitExceeded {
        limit_error(&error.to_string())
    } else {
        unsafe_error(&error.to_string())
    }
}

fn io_error(message: &str) -> GenerateError {
    super::output::io_error(message)
}

fn limit_error(message: &str) -> GenerateError {
    super::output::limit_error(message)
}
