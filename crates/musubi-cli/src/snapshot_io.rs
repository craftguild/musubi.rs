//! Bounded handle-bound snapshot input and atomic output helpers.

use std::path::Path;

#[cfg(unix)]
use std::{
    fs::File,
    io::{Read, Write},
};

#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use rustix::{
    fs::{self, AtFlags, FileType, Mode, OFlags},
    io::Errno,
};

#[cfg(unix)]
const MAX_SNAPSHOT_BYTES: usize = musubi_codegen::HardLimits::V1.generated_bytes;

pub(crate) fn read(path: &Path) -> Result<Vec<u8>, SnapshotIoError> {
    #[cfg(unix)]
    {
        read_with_hook(path, MAX_SNAPSHOT_BYTES, || {})
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(SnapshotIoError)
    }
}

#[cfg(unix)]
fn read_with_hook(
    path: &Path,
    maximum: usize,
    after_metadata: impl FnOnce(),
) -> Result<Vec<u8>, SnapshotIoError> {
    let descriptor = fs::open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(invalid)?;
    let metadata = fs::fstat(&descriptor).map_err(invalid)?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
        return Err(SnapshotIoError);
    }
    let length = usize::try_from(metadata.st_size).map_err(invalid)?;
    if length > maximum {
        return Err(SnapshotIoError);
    }
    after_metadata();
    let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::with_capacity(length);
    File::from(descriptor)
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(invalid)?;
    if bytes.len() > maximum {
        return Err(SnapshotIoError);
    }
    Ok(bytes)
}

pub(crate) fn write(path: &Path, bytes: &[u8]) -> Result<(), SnapshotIoError> {
    #[cfg(unix)]
    {
        write_with_hook(path, bytes, || {})
    }
    #[cfg(not(unix))]
    {
        let _ = (path, bytes);
        Err(SnapshotIoError)
    }
}

#[cfg(unix)]
fn write_with_hook(
    path: &Path,
    bytes: &[u8],
    after_parent_open: impl FnOnce(),
) -> Result<(), SnapshotIoError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SnapshotIoError);
    }
    let parent_path = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().ok_or(SnapshotIoError)?;
    let parent = fs::open(
        parent_path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(invalid)?;
    after_parent_open();
    validate_existing_target(&parent, name)?;
    let (temporary_name, descriptor) = create_temporary(&parent)?;
    let result = (|| {
        let mut temporary = File::from(descriptor);
        temporary.write_all(bytes).map_err(invalid)?;
        temporary.flush().map_err(invalid)?;
        temporary.sync_all().map_err(invalid)?;
        fs::renameat(&parent, temporary_name.as_str(), &parent, name).map_err(invalid)?;
        fs::fsync(&parent).map_err(invalid)
    })();
    if result.is_err() {
        let _ = fs::unlinkat(&parent, temporary_name.as_str(), AtFlags::empty());
    }
    result
}

#[cfg(unix)]
fn validate_existing_target<Fd: std::os::fd::AsFd>(
    parent: Fd,
    name: &std::ffi::OsStr,
) -> Result<(), SnapshotIoError> {
    match fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(metadata) if FileType::from_raw_mode(metadata.st_mode) == FileType::RegularFile => {
            Ok(())
        }
        Err(error) if error == Errno::NOENT => Ok(()),
        Ok(_) | Err(_) => Err(SnapshotIoError),
    }
}

#[cfg(unix)]
fn create_temporary<Fd: std::os::fd::AsFd>(
    parent: Fd,
) -> Result<(String, std::os::fd::OwnedFd), SnapshotIoError> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..128 {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = format!(".musubi-snapshot.{}.{sequence}", std::process::id());
        match fs::openat(
            &parent,
            name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        ) {
            Ok(descriptor) => return Ok((name, descriptor)),
            Err(error) if error == Errno::EXIST => {}
            Err(_) => return Err(SnapshotIoError),
        }
    }
    Err(SnapshotIoError)
}

#[cfg(unix)]
fn invalid<T>(_: T) -> SnapshotIoError {
    SnapshotIoError
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SnapshotIoError;

#[cfg(all(test, unix))]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn snapshot_write_replaces_regular_file_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("snapshot.json");
        write(&path, b"first").unwrap();
        write(&path, b"second").unwrap();
        assert_eq!(read(&path).unwrap(), b"second");
    }

    #[cfg(unix)]
    #[test]
    fn read_uses_one_regular_bounded_handle_across_path_changes() {
        use std::{io::Write as _, os::unix::fs::symlink};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("snapshot.json");
        fs::write(&path, b"inside").unwrap();
        let outside = directory.path().join("outside.json");
        fs::write(&outside, b"outside").unwrap();
        let bytes = read_with_hook(&path, MAX_SNAPSHOT_BYTES, || {
            fs::remove_file(&path).unwrap();
            symlink(&outside, &path).unwrap();
        })
        .unwrap();
        assert_eq!(bytes, b"inside");

        fs::remove_file(&path).unwrap();
        fs::write(&path, b"1234").unwrap();
        let result = read_with_hook(&path, 4, || {
            fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap()
                .write_all(b"5")
                .unwrap();
        });
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_io_rejects_symlinks_and_fifo_without_blocking() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target");
        fs::write(&target, b"target").unwrap();
        let link = directory.path().join("link");
        symlink(&target, &link).unwrap();
        assert!(read(&link).is_err());
        assert!(write(&link, b"replacement").is_err());
        assert_eq!(fs::read(target).unwrap(), b"target");

        let fifo = directory.path().join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert!(read(&fifo).is_err());
        assert!(write(&fifo, b"replacement").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn write_stays_with_the_opened_parent_after_path_swap() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        let retained = root.path().join("retained");
        let replacement = root.path().join("replacement");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&replacement).unwrap();
        let path = parent.join("snapshot.json");
        write_with_hook(&path, b"inside", || {
            fs::rename(&parent, &retained).unwrap();
            fs::rename(&replacement, &parent).unwrap();
        })
        .unwrap();
        assert_eq!(fs::read(retained.join("snapshot.json")).unwrap(), b"inside");
        assert!(!parent.join("snapshot.json").exists());
    }
}
