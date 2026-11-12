//! Deterministic manifest initialization with private staging and exclusive atomic installation.

use std::path::Path;

#[cfg(unix)]
use crate::output::unsafe_error;
use crate::{GenerateError, host_capability_preflight, output::io_error};

/// Initial editable config-v4 manifest; no artifact provenance or runtime identity is invented.
pub const INITIAL_MANIFEST: &str = concat!(
    "# Edit and review this manifest before generation.\n",
    "schema_version = \"musubi/config-v4\"\n\n",
    "[package]\n",
    "name = \"generated-contract-sdk\"\n",
    "version = \"0.1.0\"\n",
    "edition = \"2024\"\n\n",
    "[generation]\n",
    "output = \"generated/contract-sdk\"\n\n",
    "# Choose and edit one runtime source. No source is inferred.\n",
    "# [runtime]\n# source = \"path\"\n# root = \"../musubi-source\"\n\n",
    "# [runtime]\n# source = \"git\"\n",
    "# url = \"https://github.com/craftguild/musubi.rs.git\"\n",
    "# revision = \"<reviewed full 40-digit lowercase commit>\"\n\n",
    "# Add at least one active contract or typed-data entry.\n",
    "# Paths are relative to this manifest's directory.\n",
    "# Supply reviewed external compiler artifacts; their coherence remains unverified.\n",
    "# [[contracts]]\n",
    "# id = \"Example\"\n",
    "# kind = \"interface\"\n",
    "# abi = \"artifacts/Example.abi.json\"\n\n",
    "# For a deployable contract, use kind = \"deployable\" and also supply:\n",
    "# creation_bytecode = \"artifacts/Example.creation.json\"\n",
    "# Optional enrichment/provenance: ast, metadata; deployables may also use\n",
    "# runtime_bytecode and storage_layout. ABI alone determines wire encoding.\n\n",
    "# Typed-data requires a closed schema and independently produced Solidity vectors.\n",
    "# [[typed_data]]\n",
    "# id = \"Message\"\n",
    "# rust_module = \"message_typed_data\"\n",
    "# authority = \"closed-schema-v1\"\n",
    "# schema = \"typed-data/Message.schema.json\"\n",
    "# vectors = \"typed-data/Message.vectors.json\"\n",
);

/// Exclusively creates an editable manifest in an existing real parent directory.
///
/// # Errors
///
/// Rejects existing or raced targets of every kind. A failed private write is cleaned up before
/// returning; no SDK is generated and no existing manifest is replaced.
pub fn init_manifest(path: &Path) -> Result<(), GenerateError> {
    host_capability_preflight().map_err(|_| io_error(crate::UNSUPPORTED_HOST_MESSAGE))?;
    #[cfg(unix)]
    {
        initialize_with(path, |_| false)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(io_error(crate::UNSUPPORTED_HOST_MESSAGE))
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InitStep {
    TargetCheck,
    Write,
    Sync,
    Install,
}

#[cfg(unix)]
pub(crate) fn initialize_with(
    path: &Path,
    mut fail: impl FnMut(InitStep) -> bool,
) -> Result<(), GenerateError> {
    use crate::{
        input_fs::InputRoot,
        output_install::{Parent, SiblingDirectory, rename_no_replace},
    };
    use rustix::{
        fs::{self, AtFlags, Mode, OFlags},
        io::Errno,
    };
    use std::{ffi::OsStr, fs::File, io::Write as _};

    let name = path
        .file_name()
        .ok_or_else(|| unsafe_error("manifest target has no file name"))?;
    let root = InputRoot::open(path.parent().unwrap_or(Path::new(".")))
        .map_err(|_| unsafe_error("manifest parent must be an accessible real directory"))?;
    let parent = Parent::from_root(&root)?;
    if fail(InitStep::TargetCheck) {
        return Err(io_error("manifest initialization failed"));
    }
    match fs::statat(parent.descriptor(), name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => {}
        Ok(_) | Err(_) => return Err(unsafe_error("init requires an absent manifest target")),
    }
    let staging = SiblingDirectory::create(&parent, ".musubi-init-", crate::HardLimits::V1)?;
    let descriptor = fs::openat(
        staging.descriptor(),
        "manifest",
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(|_| io_error("initial manifest cannot be created"))?;
    let mut file = File::from(descriptor);
    let bytes = INITIAL_MANIFEST.as_bytes();
    file.write_all(&bytes[..bytes.len() / 2])
        .map_err(|_| io_error("initial manifest write failed"))?;
    if fail(InitStep::Write) {
        return Err(io_error("initial manifest write failed"));
    }
    file.write_all(&bytes[bytes.len() / 2..])
        .map_err(|_| io_error("initial manifest write failed"))?;
    if fail(InitStep::Sync) {
        return Err(io_error("initial manifest persistence failed"));
    }
    file.sync_all()
        .map_err(|_| io_error("initial manifest persistence failed"))?;
    if fail(InitStep::Install) {
        return Err(io_error("initial manifest installation failed"));
    }
    rename_no_replace(
        staging.descriptor(),
        OsStr::new("manifest"),
        parent.descriptor(),
        name,
    )
    .map_err(|_| unsafe_error("init requires an absent manifest target"))
}
