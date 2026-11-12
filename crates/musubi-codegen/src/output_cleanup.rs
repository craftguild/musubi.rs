//! Resource-bounded, handle-relative cleanup of generated sibling directories.

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _};

use rustix::fs::{self, AtFlags, Dir, FileType, Mode, OFlags};

use crate::{
    GenerateError, HardLimits,
    output::{io_error, limit_error, unsafe_error},
};

pub(crate) fn remove_contents(
    directory: &std::os::fd::OwnedFd,
    limits: HardLimits,
) -> Result<(), GenerateError> {
    let mut budget = CleanupBudget::default();
    remove_entries(directory, limits, 0, &mut budget)
}

pub(crate) fn remove_named_tree(
    parent: &std::os::fd::OwnedFd,
    name: &OsStr,
    limits: HardLimits,
) -> Result<(), GenerateError> {
    let directory = open_directory(parent, name)?;
    remove_contents(&directory, limits)?;
    fs::unlinkat(parent, name, AtFlags::REMOVEDIR)
        .map_err(|_| io_error("generated cleanup directory cannot be removed"))
}

#[derive(Default)]
struct CleanupBudget {
    directories: usize,
    files: usize,
    bytes: usize,
}

fn remove_entries(
    directory: &std::os::fd::OwnedFd,
    limits: HardLimits,
    depth: usize,
    budget: &mut CleanupBudget,
) -> Result<(), GenerateError> {
    if depth > limits.nesting {
        return Err(limit_error(
            "generated cleanup depth exceeds effective limits",
        ));
    }
    let entries = Dir::read_from(directory)
        .map_err(|_| io_error("generated cleanup directory cannot be read"))?;
    for entry in entries {
        let entry = entry.map_err(|_| io_error("generated cleanup entry cannot be read"))?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = OsStr::from_bytes(bytes);
        let metadata = fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|_| io_error("generated cleanup entry cannot be inspected"))?;
        if FileType::from_raw_mode(metadata.st_mode) == FileType::Directory {
            budget.directories = budget.directories.saturating_add(1);
            if budget.directories > limits.generated_files {
                return Err(limit_error(
                    "generated cleanup directory count exceeds effective limits",
                ));
            }
            let child = open_directory(directory, name)?;
            remove_entries(&child, limits, depth.saturating_add(1), budget)?;
            fs::unlinkat(directory, name, AtFlags::REMOVEDIR)
                .map_err(|_| io_error("generated cleanup directory cannot be removed"))?;
        } else {
            budget.files = budget.files.saturating_add(1);
            let length = usize::try_from(metadata.st_size).unwrap_or(usize::MAX);
            budget.bytes = budget.bytes.saturating_add(length);
            if budget.files > limits.generated_files || budget.bytes > limits.generated_bytes {
                return Err(limit_error(
                    "generated cleanup files exceed effective limits",
                ));
            }
            fs::unlinkat(directory, name, AtFlags::empty())
                .map_err(|_| io_error("generated cleanup file cannot be removed"))?;
        }
    }
    Ok(())
}

fn open_directory<Fd: std::os::fd::AsFd>(
    parent: Fd,
    name: &OsStr,
) -> Result<std::os::fd::OwnedFd, GenerateError> {
    fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| unsafe_error("generated cleanup directory cannot be opened safely"))
}
