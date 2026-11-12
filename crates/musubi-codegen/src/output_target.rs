//! Output authority tied to the directory handle that supplied the manifest bytes.

use std::path::PathBuf;

use crate::{GenerationOutput, input_fs::InputRoot};

/// Manifest-owned output location with its original root capability retained.
///
/// Constructed only by input loading. Cloning retains the same directory handle; later pathname
/// changes cannot redirect generation to another manifest root.
#[derive(Clone, Debug)]
pub struct OutputTarget {
    root: InputRoot,
    relative: GenerationOutput,
}

impl OutputTarget {
    pub(crate) fn new(root: InputRoot, relative: GenerationOutput) -> Self {
        Self { root, relative }
    }

    /// Returns the validated manifest-relative generated crate location.
    #[must_use]
    pub fn relative(&self) -> &GenerationOutput {
        &self.relative
    }

    pub(crate) fn conceptual_path(&self) -> PathBuf {
        self.root.path().join(self.relative.as_path())
    }

    pub(crate) fn configuration_root(&self) -> &std::path::Path {
        self.root.path()
    }

    #[cfg(unix)]
    pub(crate) fn resolve(&self) -> Result<ResolvedOutput, crate::GenerateError> {
        use crate::output::unsafe_error;
        use rustix::{
            fs::{self, Mode, OFlags},
            io::Errno,
        };
        let mut components = self
            .relative
            .as_path()
            .components()
            .map(|component| component.as_os_str().to_owned())
            .collect::<Vec<_>>();
        let name = components
            .pop()
            .ok_or_else(|| unsafe_error("generated target has no name"))?;
        let mut parent = crate::output_install::Parent::from_root(&self.root)?;
        let mut components = components.into_iter();
        while let Some(component) = components.next() {
            match fs::openat(
                parent.descriptor(),
                &component,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(descriptor) => parent = parent.child(&component, descriptor),
                Err(Errno::NOENT) => {
                    return Ok(ResolvedOutput {
                        parent,
                        name,
                        missing: std::iter::once(component).chain(components).collect(),
                    });
                }
                Err(_) => {
                    return Err(unsafe_error(
                        "generated parent must remain below the manifest root without links",
                    ));
                }
            }
        }
        Ok(ResolvedOutput {
            parent,
            name,
            missing: Vec::new(),
        })
    }
}

/// Retains the deepest existing parent even when later configured parent directories are absent.
/// Cargo validation cannot cause a pathname swap to rebind that already acquired capability.
#[cfg(unix)]
pub(crate) struct ResolvedOutput {
    parent: crate::output_install::Parent,
    name: std::ffi::OsString,
    missing: Vec<std::ffi::OsString>,
}

#[cfg(unix)]
impl ResolvedOutput {
    pub(crate) fn existing(&self) -> Option<(&crate::output_install::Parent, &std::ffi::OsStr)> {
        self.missing
            .is_empty()
            .then_some((&self.parent, self.name.as_os_str()))
    }

    pub(crate) fn create_parent(
        mut self,
    ) -> Result<(crate::output_install::Parent, std::ffi::OsString), crate::GenerateError> {
        use crate::output::{io_error, unsafe_error};
        use rustix::{
            fs::{self, Mode, OFlags},
            io::Errno,
        };
        for component in self.missing {
            match fs::mkdirat(self.parent.descriptor(), &component, Mode::RWXU) {
                Ok(()) | Err(Errno::EXIST) => {}
                Err(_) => return Err(io_error("generated parent cannot be created")),
            }
            let descriptor = fs::openat(
                self.parent.descriptor(),
                &component,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| {
                unsafe_error(
                    "generated parent must remain below the retained manifest parent without links",
                )
            })?;
            self.parent = self.parent.child(&component, descriptor);
        }
        Ok((self.parent, self.name))
    }
}

#[cfg(test)]
pub(crate) fn for_test(path: &std::path::Path) -> OutputTarget {
    let mut parent = path.parent().unwrap();
    while !parent.is_dir() {
        parent = parent.parent().unwrap();
    }
    let relative = path
        .strip_prefix(parent)
        .unwrap()
        .components()
        .map(|component| component.as_os_str().to_str().unwrap())
        .collect::<Vec<_>>()
        .join("/");
    let value: crate::GenerationConfig = toml::from_str(&format!("output = {relative:?}")).unwrap();
    OutputTarget::new(InputRoot::for_test(parent), value.output)
}
