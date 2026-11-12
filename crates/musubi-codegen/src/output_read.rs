//! No-follow root traversal and metadata-stable, bounded reads from retained file handles.

use std::{
    ffi::OsStr,
    fs::File,
    io::Read as _,
    os::fd::{AsFd, OwnedFd},
    path::{Component, Path, PathBuf},
};

use rustix::fs::{self, Dir, FileType, Mode, OFlags, Stat};

use crate::{
    GenerateError,
    output::{io_error, limit_error, unsafe_error},
};

/// A depth-first frame keeps open handles proportional to depth, not sibling directory count.
pub(crate) struct Directory {
    pub(crate) descriptor: OwnedFd,
    pub(crate) entries: Dir,
    pub(crate) relative: PathBuf,
    pub(crate) depth: usize,
    pub(crate) count: usize,
    before: Stat,
}

impl Directory {
    pub(crate) fn new(
        descriptor: OwnedFd,
        relative: PathBuf,
        depth: usize,
    ) -> Result<Self, GenerateError> {
        let before = metadata(&descriptor)?;
        let entries = Dir::read_from(&descriptor)
            .map_err(|_| io_error("generated directory cannot be read"))?;
        Ok(Self {
            descriptor,
            entries,
            relative,
            depth,
            count: 0,
            before,
        })
    }

    pub(crate) fn finish(self) -> Result<(), GenerateError> {
        if self.count == 0 && !self.relative.as_os_str().is_empty() {
            return Err(unsafe_error("generated tree contains an empty directory"));
        }
        unchanged(&self.before, &metadata(&self.descriptor)?)
    }
}

pub(crate) fn root(path: &Path) -> Result<OwnedFd, GenerateError> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let anchor = if path.is_absolute() { "/" } else { "." };
    let mut directory = fs::open(anchor, flags, Mode::empty())
        .map_err(|_| unsafe_error("generated root cannot be opened"))?;
    for component in path.components() {
        let name = match component {
            Component::Normal(name) => name,
            Component::ParentDir => OsStr::new(".."),
            Component::CurDir | Component::RootDir => continue,
            Component::Prefix(_) => return Err(unsafe_error("generated root is not supported")),
        };
        directory = fs::openat(&directory, name, flags, Mode::empty())
            .map_err(|_| unsafe_error("generated root must have real directory ancestors"))?;
    }
    Ok(directory)
}

pub(crate) fn entry(directory: impl AsFd, name: &OsStr) -> Result<OwnedFd, GenerateError> {
    fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| unsafe_error("generated entry cannot be opened without following links"))
}

pub(crate) fn metadata(descriptor: impl AsFd) -> Result<Stat, GenerateError> {
    fs::fstat(descriptor).map_err(|_| io_error("generated entry metadata cannot be read"))
}

/// Ignore atime, which our read may update. Size, identity and change/write times must stay fixed.
pub(crate) fn unchanged(before: &Stat, after: &Stat) -> Result<(), GenerateError> {
    if before.st_dev != after.st_dev
        || before.st_ino != after.st_ino
        || before.st_mode != after.st_mode
        || before.st_size != after.st_size
        || before.st_mtime != after.st_mtime
        || before.st_mtime_nsec != after.st_mtime_nsec
        || before.st_ctime != after.st_ctime
        || before.st_ctime_nsec != after.st_ctime_nsec
    {
        return Err(unsafe_error("generated entry changed during inspection"));
    }
    Ok(())
}

pub(crate) fn bounded(
    mut file: File,
    before: &Stat,
    maximum: usize,
) -> Result<Vec<u8>, GenerateError> {
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile {
        return Err(unsafe_error("generated input must be a regular file"));
    }
    let length = usize::try_from(before.st_size)
        .map_err(|_| limit_error("generated file size exceeds limits"))?;
    if length > maximum {
        return Err(limit_error("generated file size exceeds limits"));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(length as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| io_error("generated file cannot be read"))?;
    if bytes.len() > maximum {
        return Err(limit_error("generated file size exceeds limits"));
    }
    unchanged(before, &metadata(&file)?)?;
    if bytes.len() != length {
        return Err(unsafe_error("generated file changed during inspection"));
    }
    Ok(bytes)
}
