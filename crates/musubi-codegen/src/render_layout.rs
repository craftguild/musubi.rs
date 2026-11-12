//! Canonical private owner paths and collision detection, independent of ABI item positions.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::{RenderError, names::rust_snake, render::render_error};

/// One private Rust module registered directly below its public authority facade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticOwner {
    /// Normal private file path relative to the authority directory.
    pub path: String,
    /// Exact private Rust module identifier declared by the authority facade.
    pub module: String,
}

pub(crate) type Leaf = SemanticOwner;

/// Detects both shortened identity collisions and case-insensitive file collisions per authority.
#[derive(Default)]
pub(crate) struct Naming {
    digests: BTreeMap<(String, [u8; 16]), [u8; 32]>,
    paths: BTreeMap<String, String>,
}

impl Naming {
    pub(crate) fn owner(
        &mut self,
        kind: &str,
        name: &str,
        identity: &[u8],
    ) -> Result<Leaf, RenderError> {
        self.owner_with_digest(kind, name, identity, |full| full[..16].try_into().unwrap())
    }

    /// Injection is private to the renderer; tests can exercise otherwise infeasible collisions.
    fn owner_with_digest(
        &mut self,
        kind: &str,
        name: &str,
        identity: &[u8],
        shorten: fn([u8; 32]) -> [u8; 16],
    ) -> Result<Leaf, RenderError> {
        let mut encoder = Sha256::new();
        for part in [
            b"musubi/semantic-owner/v1".as_slice(),
            kind.as_bytes(),
            identity,
        ] {
            encoder.update((part.len() as u64).to_be_bytes());
            encoder.update(part);
        }
        let full: [u8; 32] = encoder.finalize().into();
        let digest = shorten(full);
        if self
            .digests
            .insert((kind.into(), digest), full)
            .is_some_and(|old| old != full)
        {
            return Err(render_error("private owner identity digest collision"));
        }
        // Normalization emits ASCII; truncation is a byte limit, never a line-count decision.
        let stem = rust_snake(name);
        let stem = stem.strip_prefix("r#").unwrap_or(&stem);
        let stem = &stem[..stem.len().min(48)];
        let suffix = hex::encode(digest);
        let basename = format!("{stem}_{suffix}");
        let path = format!("{kind}/{basename}.rs");
        if self
            .paths
            .insert(path.to_ascii_lowercase(), path.clone())
            .is_some()
        {
            return Err(render_error("private owner file path collision"));
        }
        Ok(Leaf {
            path,
            module: format!("__evm_{kind}_{basename}"),
        })
    }
}

#[cfg(test)]
mod tests {
    //! Independent fixed naming vectors and shortened-digest collision rejection.

    use super::*;

    #[test]
    fn semantic_owner_naming_is_bounded_and_collision_checked() {
        let mut names = Naming::default();
        let leaf = names
            .owner("operations", "transfer", b"transfer(address,uint256)")
            .unwrap();
        assert_eq!(
            leaf.path,
            "operations/transfer_95f7819c0d5083729310eddad926ab8b.rs"
        );
        let leaf = names.owner("events", &"A".repeat(256), b"A()").unwrap();
        assert_eq!(leaf.path.split('/').nth(1).unwrap().len(), 48 + 1 + 32 + 3);
        let keyword = names.owner("events", "type", b"type()").unwrap();
        assert!(keyword.path.starts_with("events/type_"));
        assert!(!keyword.module.contains('#'));
        let mut names = Naming::default();
        names
            .owner_with_digest("operations", "same", b"same()", |_| [0; 16])
            .unwrap();
        assert!(
            names
                .owner_with_digest("operations", "different", b"different()", |_| [0; 16])
                .is_err()
        );
    }
}
