//! Handle-relative staged generated file creation and persistence.

use crate::{
    GenerateError,
    output::{io_error, unsafe_error},
};
use rustix::{
    fs::{self, Mode, OFlags},
    io::{Errno, dup},
};
use std::{ffi::OsStr, fs::File, io::Write as _, path::Path};

pub(crate) fn write_file(
    root: &std::os::fd::OwnedFd,
    path: &Path,
    bytes: &[u8],
) -> Result<(), GenerateError> {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_owned())
        .collect::<Vec<_>>();
    let (file_name, parents) = components
        .split_last()
        .ok_or_else(|| unsafe_error("generated file has no name"))?;
    let mut directory = dup(root).map_err(|_| io_error("staging handle cannot be duplicated"))?;
    for component in parents {
        match fs::mkdirat(&directory, component, Mode::RWXU) {
            Ok(()) => {}
            Err(error) if error == Errno::EXIST => {}
            Err(_) => return Err(io_error("generated directory cannot be created")),
        }
        directory = open_directory(&directory, component)?;
    }
    let descriptor = fs::openat(
        &directory,
        file_name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(|_| io_error("generated file cannot be created"))?;
    let mut file = File::from(descriptor);
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| io_error("generated file cannot be persisted"))
}

pub(crate) fn open_directory<Fd: std::os::fd::AsFd>(
    parent: Fd,
    name: &OsStr,
) -> Result<std::os::fd::OwnedFd, GenerateError> {
    fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| unsafe_error("generated directory cannot be opened safely"))
}
