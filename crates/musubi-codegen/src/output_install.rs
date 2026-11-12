//! Handle-relative generated-tree staging, replacement, and rollback.

use std::{
    ffi::OsStr,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(test)]
use rustix::fs::OFlags;
#[cfg(test)]
use std::path::Path;

use rustix::{
    fs::{self, AtFlags, Mode},
    io::{Errno, dup},
};

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "redox"
))]
use rustix::fs::RenameFlags;

use crate::{
    GenerateError, GeneratedTree, HardLimits,
    output::{io_error, unsafe_error},
    output_cleanup, output_fs,
    output_write::{open_directory, write_file},
};

pub(crate) struct Parent {
    path: PathBuf,
    descriptor: std::os::fd::OwnedFd,
}

impl Parent {
    pub(crate) fn from_root(root: &crate::input_fs::InputRoot) -> Result<Self, GenerateError> {
        Ok(Self {
            path: root.path().to_path_buf(),
            descriptor: dup(root.descriptor())
                .map_err(|_| io_error("manifest root handle cannot be retained"))?,
        })
    }

    pub(crate) fn descriptor(&self) -> &std::os::fd::OwnedFd {
        &self.descriptor
    }

    pub(crate) fn child(&self, name: &OsStr, descriptor: std::os::fd::OwnedFd) -> Self {
        Self {
            path: self.path.join(name),
            descriptor,
        }
    }

    #[cfg(test)]
    pub(crate) fn open(path: &Path) -> Result<Self, GenerateError> {
        let canonical = path
            .canonicalize()
            .map_err(|_| io_error("generated target parent cannot be resolved"))?;
        let descriptor = fs::open(
            &canonical,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| unsafe_error("generated target parent must be a real directory"))?;
        Ok(Self {
            path: canonical,
            descriptor,
        })
    }

    #[cfg(test)]
    pub(crate) fn child_path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub(crate) fn validate_existing(
        &self,
        name: &OsStr,
        limits: HardLimits,
    ) -> Result<bool, GenerateError> {
        output_fs::validate_existing_at(&self.descriptor, name, limits)
    }
}

pub(crate) struct SiblingDirectory {
    parent: std::os::fd::OwnedFd,
    name: Option<String>,
    descriptor: std::os::fd::OwnedFd,
    cleanup_limits: HardLimits,
}

impl SiblingDirectory {
    pub(crate) fn create(
        parent: &Parent,
        prefix: &str,
        cleanup_limits: HardLimits,
    ) -> Result<Self, GenerateError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..128 {
            let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
            let name = format!("{prefix}{}-{sequence}", std::process::id());
            match fs::mkdirat(&parent.descriptor, name.as_str(), Mode::RWXU) {
                Ok(()) => {
                    let descriptor = open_directory(&parent.descriptor, OsStr::new(&name))?;
                    return Ok(Self {
                        parent: dup(&parent.descriptor)
                            .map_err(|_| io_error("generated parent handle cannot be retained"))?,
                        name: Some(name),
                        descriptor,
                        cleanup_limits,
                    });
                }
                Err(error) if error == Errno::EXIST => {}
                Err(_) => return Err(io_error("generated sibling directory cannot be created")),
            }
        }
        Err(io_error(
            "generated sibling directory name space is exhausted",
        ))
    }

    pub(crate) fn name(&self) -> &str {
        self.name.as_deref().expect("live sibling has a name")
    }

    pub(crate) fn descriptor(&self) -> &std::os::fd::OwnedFd {
        &self.descriptor
    }

    pub(crate) fn write_tree(&self, tree: &GeneratedTree) -> Result<(), GenerateError> {
        for (path, contents) in tree.file_entries() {
            write_file(&self.descriptor, path, contents)?;
        }
        fs::fsync(&self.descriptor)
            .map_err(|_| io_error("generated staging directory cannot be persisted"))
    }

    fn preserve(mut self) {
        self.name = None;
    }

    fn installed(mut self) {
        self.name = None;
    }

    fn remove_previous(mut self) -> Result<(), GenerateError> {
        let name = self.name.take().expect("live sibling has a name");
        output_cleanup::remove_named_tree(
            &self.descriptor,
            OsStr::new("previous"),
            self.cleanup_limits,
        )?;
        fs::unlinkat(&self.parent, name, AtFlags::REMOVEDIR)
            .map_err(|_| io_error("generated cleanup directory cannot be removed"))
    }
}

impl Drop for SiblingDirectory {
    fn drop(&mut self) {
        if let Some(name) = self.name.take() {
            let _ = output_cleanup::remove_contents(&self.descriptor, self.cleanup_limits);
            let _ = fs::unlinkat(&self.parent, name.as_str(), AtFlags::REMOVEDIR);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReplaceStep {
    Backup,
    Install,
    Rollback,
    RecoveryWrite,
    Cleanup,
    RecoveryRemove,
}

pub(crate) fn replace(
    parent: &Parent,
    output_name: &OsStr,
    staging: SiblingDirectory,
    limits: HardLimits,
    operation: crate::WriteOperation,
) -> Result<(), GenerateError> {
    replace_with(parent, output_name, staging, limits, operation, |_| false)
}

pub(crate) fn replace_with(
    parent: &Parent,
    output_name: &OsStr,
    staging: SiblingDirectory,
    limits: HardLimits,
    operation: crate::WriteOperation,
    mut fail: impl FnMut(ReplaceStep) -> bool,
) -> Result<(), GenerateError> {
    validate_operation(parent, output_name, limits, operation)?;
    if operation == crate::WriteOperation::Create {
        rename(
            &parent.descriptor,
            OsStr::new(staging.name()),
            &parent.descriptor,
            output_name,
            ReplaceStep::Install,
            &mut fail,
        )?;
        staging.installed();
        return Ok(());
    }
    let backup = SiblingDirectory::create(parent, ".musubi-backup-", limits)?;
    rename(
        &parent.descriptor,
        output_name,
        &backup.descriptor,
        OsStr::new("previous"),
        ReplaceStep::Backup,
        &mut fail,
    )?;
    let previous = output_fs::read_tree_at(&backup.descriptor, OsStr::new("previous"), limits);
    let ownership = previous
        .as_ref()
        .ok()
        .and_then(Option::as_ref)
        .is_some_and(|files| crate::validate_owned_files(files, limits).is_ok());
    if !ownership {
        let error = match previous {
            Err(error) => error,
            Ok(None) => unsafe_error("backed-up generated target disappeared"),
            Ok(Some(_)) => unsafe_error("backed-up generation marker is invalid"),
        };
        if rollback(&backup, parent, output_name, &mut fail).is_err() {
            backup.preserve();
            return Err(super::output::ownership_rollback_error(error));
        }
        return Err(error);
    }
    if rename(
        &parent.descriptor,
        OsStr::new(staging.name()),
        &parent.descriptor,
        output_name,
        ReplaceStep::Install,
        &mut fail,
    )
    .is_err()
    {
        if rollback(&backup, parent, output_name, &mut fail).is_err() {
            backup.preserve();
            return Err(super::output::replacement_rollback_error());
        }
        return Err(io_error(
            "generated replacement failed; previous tree was restored",
        ));
    }
    staging.installed();
    let previous = previous
        .expect("validated previous snapshot")
        .expect("existing previous SDK");
    let recovery = match crate::output_recovery::RecoveryFile::create(
        parent,
        &previous,
        limits,
        fail(ReplaceStep::RecoveryWrite),
    ) {
        Ok(recovery) => recovery,
        Err(mut error) => {
            backup.preserve();
            error.diagnostics.push(crate::Diagnostic::error(crate::DiagnosticCode::InvalidInput,
                "previous SDK preserved in a sibling backup directory; recovery preparation failed before cleanup"));
            return Err(error);
        }
    };
    if fail(ReplaceStep::Cleanup) {
        backup.preserve();
        return Err(recovery.retain_error(io_error("previous SDK backup cleanup failed")));
    }
    backup
        .remove_previous()
        .map_err(|error| recovery.retain_error(error))?;
    recovery.remove(fail(ReplaceStep::RecoveryRemove))
}

pub(crate) fn validate_operation(
    parent: &Parent,
    name: &OsStr,
    limits: HardLimits,
    operation: crate::WriteOperation,
) -> Result<(), GenerateError> {
    match operation {
        crate::WriteOperation::Create => {
            match fs::statat(parent.descriptor(), name, AtFlags::SYMLINK_NOFOLLOW) {
                Err(Errno::NOENT) => Ok(()),
                Ok(_) | Err(_) => Err(unsafe_error("generate requires an absent output target")),
            }
        }
        crate::WriteOperation::Update => {
            if parent.validate_existing(name, limits)? {
                Ok(())
            } else {
                Err(unsafe_error("update requires an existing marker-owned SDK"))
            }
        }
    }
}

fn rollback(
    backup: &SiblingDirectory,
    parent: &Parent,
    output_name: &OsStr,
    fail: &mut impl FnMut(ReplaceStep) -> bool,
) -> Result<(), GenerateError> {
    rename(
        &backup.descriptor,
        OsStr::new("previous"),
        &parent.descriptor,
        output_name,
        ReplaceStep::Rollback,
        fail,
    )
}

fn rename(
    from_parent: &std::os::fd::OwnedFd,
    from: &OsStr,
    to_parent: &std::os::fd::OwnedFd,
    to: &OsStr,
    step: ReplaceStep,
    fail: &mut impl FnMut(ReplaceStep) -> bool,
) -> Result<(), GenerateError> {
    if fail(step) {
        return Err(io_error("injected generated rename failure"));
    }
    rename_no_replace(from_parent, from, to_parent, to)
        .map_err(|_| io_error("generated directory rename failed"))
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "redox"
))]
pub(crate) fn rename_no_replace(
    from_parent: &std::os::fd::OwnedFd,
    from: &OsStr,
    to_parent: &std::os::fd::OwnedFd,
    to: &OsStr,
) -> rustix::io::Result<()> {
    fs::renameat_with(from_parent, from, to_parent, to, RenameFlags::NOREPLACE)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "redox"
)))]
pub(crate) fn rename_no_replace(
    _from_parent: &std::os::fd::OwnedFd,
    _from: &OsStr,
    _to_parent: &std::os::fd::OwnedFd,
    _to: &OsStr,
) -> rustix::io::Result<()> {
    Err(Errno::NOSYS)
}
