//! Single-file recovery evidence survives every fallible recursive backup-cleanup step.
//!
//! After the replacement is installed, the validated previous file snapshot is persisted here
//! before recursive cleanup starts. Its final removal is one atomic unlink: failure leaves all
//! previous file bytes available to the repository recovery extractor.

use crate::{GenerateError, HardLimits, output::io_error, output_install::Parent};
use rustix::{
    fs::{self, AtFlags, Mode, OFlags},
    io::{Errno, dup},
};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Write as _,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const MAGIC: &[u8] = b"musubi/recovery-v1\n";

pub(crate) struct RecoveryFile {
    parent: std::os::fd::OwnedFd,
    name: String,
}

impl RecoveryFile {
    pub(crate) fn create(
        parent: &Parent,
        files: &BTreeMap<PathBuf, Vec<u8>>,
        limits: HardLimits,
        fail_write: bool,
    ) -> Result<Self, GenerateError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        crate::validate_owned_files(files, limits).map_err(crate::output_fs::ownership_error)?;
        let parent =
            dup(parent.descriptor()).map_err(|_| io_error("recovery parent cannot be retained"))?;
        for _ in 0..128 {
            let name = format!(
                ".musubi-recovery-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let descriptor = match fs::openat(
                &parent,
                name.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            ) {
                Ok(descriptor) => descriptor,
                Err(Errno::EXIST) => continue,
                Err(_) => return Err(io_error("previous SDK recovery file cannot be created")),
            };
            let mut file = File::from(descriptor);
            let result = write_snapshot(&mut file, files, limits, fail_write)
                .and_then(|()| file.sync_all())
                .and_then(|()| fs::fsync(&parent).map_err(std::io::Error::from));
            if result.is_err() {
                let _ = fs::unlinkat(&parent, name.as_str(), AtFlags::empty());
                return Err(io_error("previous SDK recovery file cannot be persisted"));
            }
            return Ok(Self { parent, name });
        }
        Err(io_error("previous SDK recovery name space is exhausted"))
    }

    pub(crate) fn retain_error(&self, mut error: GenerateError) -> GenerateError {
        error.diagnostics.push(crate::Diagnostic::error(crate::DiagnosticCode::InvalidInput,
            format!("previous SDK file bytes retained in sibling recovery file `{}`; use the reviewed recovery extractor", self.name)));
        error
    }

    pub(crate) fn remove(self, fail: bool) -> Result<(), GenerateError> {
        if fail || fs::unlinkat(&self.parent, self.name.as_str(), AtFlags::empty()).is_err() {
            return Err(self.retain_error(io_error("previous SDK recovery cleanup failed")));
        }
        Ok(())
    }
}

fn write_snapshot(
    file: &mut File,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    limits: HardLimits,
    fail: bool,
) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt as _;
    // Lengths are big-endian; file bytes and native Unix relative path bytes are unmodified.
    file.write_all(MAGIC)?;
    file.write_all(&(files.len() as u64).to_be_bytes())?;
    if fail {
        return Err(std::io::Error::other("injected recovery write failure"));
    }
    for (path, bytes) in files {
        let name = path.as_os_str().as_bytes();
        crate::ownership_path::validate(
            path.to_str()
                .ok_or_else(|| std::io::Error::other("recovery path is not UTF-8"))?,
            limits,
        )
        .map_err(|_| std::io::Error::other("recovery path is invalid"))?;
        let length = u32::try_from(name.len())
            .map_err(|_| std::io::Error::other("recovery path is too long"))?;
        file.write_all(&length.to_be_bytes())?;
        file.write_all(&(bytes.len() as u64).to_be_bytes())?;
        file.write_all(name)?;
        file.write_all(bytes)?;
    }
    Ok(())
}
