//! Canonical owned Rust type identity and anonymous tuple registry construction.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use musubi_core::{AbiField, AbiType, ContractAbi};

use crate::{
    Diagnostic, DiagnosticCode, Enrichment, EnrichmentKind, HardLimits,
    ast_internal::matching_internal_base,
    names::{rust_field, rust_variant},
    owned_encoding::{
        IdentityEncoder, OwnedTypeIdentity, array_identity, primitive_identity, primitive_rust_type,
    },
};

/// One direct field of a pre-resolved anonymous tuple declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OwnedField {
    /// Exact ABI field name used in generated documentation.
    pub(crate) abi_name: String,
    /// Normalized generated Rust field name.
    pub(crate) rust_name: String,
    /// Fully resolved generated Rust field type.
    pub(crate) rust_type: String,
}

/// One anonymous tuple declaration resolved before validation and rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OwnedTuple {
    /// Full identity digest used to detect shortened-digest collisions.
    pub(crate) identity: OwnedTypeIdentity,
    /// Deterministic generated Rust type name.
    pub(crate) rust_name: String,
    /// Direct fields without cloned recursive ABI subtrees.
    pub(crate) fields: Vec<OwnedField>,
}

/// Contract-local owned identity lookup and deduplicated anonymous tuple declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OwnedTypeRegistry {
    names: HashMap<OwnedTypeIdentity, String>,
    tuples: BTreeMap<String, OwnedTuple>,
    enrichment_identities: BTreeMap<String, OwnedTypeIdentity>,
    rust_types: HashMap<AbiField, String>,
    maximum_nesting: usize,
}

impl OwnedTypeRegistry {
    /// Resolves every owned type identity in the bounded ABI IR exactly once.
    pub(crate) fn build(
        abi: &ContractAbi,
        enrichments: &[Enrichment],
        limits: HardLimits,
    ) -> Result<Self, Diagnostic> {
        Builder::new(enrichments, limits, production_short_digest).build(abi)
    }

    #[cfg(test)]
    pub(crate) fn build_with_digest(
        abi: &ContractAbi,
        enrichments: &[Enrichment],
        limits: HardLimits,
        digest: fn(&[u8]) -> [u8; 16],
    ) -> Result<Self, Diagnostic> {
        Builder::new(enrichments, limits, digest).build(abi)
    }

    /// Returns declarations in deterministic generated-name order.
    pub(crate) fn tuples(&self) -> impl Iterator<Item = &OwnedTuple> {
        self.tuples.values()
    }

    /// Returns the pre-resolved Rust type for one directly rendered ABI field.
    pub(crate) fn rust_type(&self, field: &AbiField) -> Option<&str> {
        self.rust_types.get(field).map(String::as_str)
    }

    /// Returns the number of ABI-reachable AST enrichments sealed into this registry.
    pub(crate) fn enrichment_count(&self) -> usize {
        self.enrichment_identities.len()
    }

    #[cfg(test)]
    pub(crate) fn anonymous_name(&self, ty: &AbiType, enrichment: Option<&str>) -> Option<&str> {
        let identity = self.existing_identity(ty, enrichment)?;
        self.names.get(&identity).map(String::as_str)
    }

    /// Returns a validated AST enrichment base for one complete ABI field occurrence.
    #[cfg(test)]
    pub(crate) fn resolved_enrichment<'a>(
        &'a self,
        ty: &AbiType,
        internal: Option<&'a str>,
    ) -> Option<&'a str> {
        let base = matching_internal_base(ty, internal?, self.maximum_nesting)?;
        self.enrichment_identities
            .contains_key(base)
            .then_some(base)
    }

    #[cfg(test)]
    pub(crate) fn identity(
        &self,
        ty: &AbiType,
        internal: Option<&str>,
    ) -> Option<OwnedTypeIdentity> {
        let enrichment = self.resolved_enrichment(ty, internal);
        self.existing_identity(ty, enrichment)
    }

    #[cfg(test)]
    fn existing_identity(
        &self,
        ty: &AbiType,
        enrichment: Option<&str>,
    ) -> Option<OwnedTypeIdentity> {
        match ty {
            AbiType::Array(element) => Some(array_identity(
                2,
                None,
                self.existing_identity(element, enrichment)?,
            )),
            AbiType::FixedArray(element, length) => Some(array_identity(
                3,
                Some(*length),
                self.existing_identity(element, enrichment)?,
            )),
            _ => match enrichment {
                Some(name) => self.enrichment_identities.get(name).copied(),
                None => match ty {
                    AbiType::Tuple(fields) => self.existing_tuple_identity(fields),
                    _ => Some(primitive_identity(ty)),
                },
            },
        }
    }

    #[cfg(test)]
    fn existing_tuple_identity(&self, fields: &[AbiField]) -> Option<OwnedTypeIdentity> {
        let mut encoder = IdentityEncoder::new(4);
        encoder.usize(fields.len());
        for (index, field) in fields.iter().enumerate() {
            encoder.bytes(field.name.as_bytes());
            encoder.bytes(rust_field(&field.name, index).as_bytes());
            let enrichment = self.resolved_enrichment(&field.ty, field.internal_type.as_deref());
            encoder.identity(self.existing_identity(&field.ty, enrichment)?);
        }
        Some(encoder.finish())
    }
}

#[derive(Clone)]
struct ResolvedType {
    identity: OwnedTypeIdentity,
    rust_type: String,
}

struct ResolvedField {
    identity: OwnedTypeIdentity,
    field: OwnedField,
}

struct Builder<'a> {
    enrichments: BTreeMap<&'a str, &'a Enrichment>,
    resolving_enrichments: BTreeSet<String>,
    short_digest: fn(&[u8]) -> [u8; 16],
    registry: OwnedTypeRegistry,
}

impl<'a> Builder<'a> {
    fn new(
        enrichments: &'a [Enrichment],
        limits: HardLimits,
        short_digest: fn(&[u8]) -> [u8; 16],
    ) -> Self {
        Self {
            enrichments: enrichments
                .iter()
                .map(|item| (item.internal_type.as_str(), item))
                .collect(),
            resolving_enrichments: BTreeSet::new(),
            short_digest,
            registry: OwnedTypeRegistry {
                names: HashMap::new(),
                tuples: BTreeMap::new(),
                enrichment_identities: BTreeMap::new(),
                rust_types: HashMap::new(),
                maximum_nesting: limits.nesting,
            },
        }
    }

    fn build(mut self, abi: &ContractAbi) -> Result<OwnedTypeRegistry, Diagnostic> {
        for field in all_fields(abi) {
            let resolved = self.resolve_field(field)?;
            self.registry
                .rust_types
                .entry(field.clone())
                .or_insert(resolved.rust_type);
        }
        Ok(self.registry)
    }

    fn resolve(
        &mut self,
        ty: &AbiType,
        enrichment: Option<&str>,
    ) -> Result<ResolvedType, Diagnostic> {
        match ty {
            AbiType::Array(element) => {
                let child = self.resolve(element, enrichment)?;
                Ok(ResolvedType {
                    identity: array_identity(2, None, child.identity),
                    rust_type: format!("Vec<{}>", child.rust_type),
                })
            }
            AbiType::FixedArray(element, length) => {
                let child = self.resolve(element, enrichment)?;
                Ok(ResolvedType {
                    identity: array_identity(3, Some(*length), child.identity),
                    rust_type: format!("[{}; {length}]", child.rust_type),
                })
            }
            _ => match enrichment {
                Some(name) => self.enrichment_type(name),
                None => match ty {
                    AbiType::Tuple(fields) => self.anonymous_tuple(fields),
                    _ => Ok(ResolvedType {
                        identity: primitive_identity(ty),
                        rust_type: primitive_rust_type(ty),
                    }),
                },
            },
        }
    }

    fn resolve_field(&mut self, field: &AbiField) -> Result<ResolvedType, Diagnostic> {
        let enrichment = field.internal_type.as_deref().and_then(|internal| {
            matching_internal_base(&field.ty, internal, self.registry.maximum_nesting)
                .filter(|base| self.enrichments.contains_key(base))
        });
        self.resolve(&field.ty, enrichment)
    }

    fn resolve_fields(&mut self, fields: &[AbiField]) -> Result<Vec<ResolvedField>, Diagnostic> {
        fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let resolved = self.resolve_field(field)?;
                Ok(ResolvedField {
                    identity: resolved.identity,
                    field: OwnedField {
                        abi_name: field.name.clone(),
                        rust_name: rust_field(&field.name, index),
                        rust_type: resolved.rust_type,
                    },
                })
            })
            .collect()
    }

    fn anonymous_tuple(&mut self, fields: &[AbiField]) -> Result<ResolvedType, Diagnostic> {
        let fields = self.resolve_fields(fields)?;
        let identity = fields_identity(4, &fields);
        let rust_name = format!("Tuple{}", hex::encode((self.short_digest)(&identity.0)));
        if let Some(previous) = self.registry.tuples.get(&rust_name) {
            if previous.identity != identity {
                return Err(collision());
            }
        } else {
            self.registry.names.insert(identity, rust_name.clone());
            self.registry.tuples.insert(
                rust_name.clone(),
                OwnedTuple {
                    identity,
                    rust_name: rust_name.clone(),
                    fields: fields.into_iter().map(|field| field.field).collect(),
                },
            );
        }
        Ok(ResolvedType {
            identity,
            rust_type: rust_name,
        })
    }

    fn enrichment_type(&mut self, name: &str) -> Result<ResolvedType, Diagnostic> {
        let enrichment = self.enrichments.get(name).copied().ok_or_else(collision)?;
        if let Some(identity) = self.registry.enrichment_identities.get(name).copied() {
            return Ok(ResolvedType {
                identity,
                rust_type: enrichment.rust_name.clone(),
            });
        }
        if !self.resolving_enrichments.insert(name.to_owned()) {
            return Err(collision());
        }
        let identity = self.enrichment_identity(enrichment)?;
        self.resolving_enrichments.remove(name);
        self.registry
            .enrichment_identities
            .insert(name.to_owned(), identity);
        Ok(ResolvedType {
            identity,
            rust_type: enrichment.rust_name.clone(),
        })
    }

    fn enrichment_identity(
        &mut self,
        enrichment: &Enrichment,
    ) -> Result<OwnedTypeIdentity, Diagnostic> {
        let mut encoder = IdentityEncoder::new(5);
        encoder.bytes(enrichment.internal_type.as_bytes());
        encoder.bytes(enrichment.rust_name.as_bytes());
        match &enrichment.kind {
            EnrichmentKind::Enum {
                variants,
                underlying,
            } => {
                encoder.byte(1);
                encoder.usize(variants.len());
                for variant in variants {
                    encoder.bytes(variant.as_bytes());
                    encoder.bytes(rust_variant(variant).as_bytes());
                }
                encoder.identity(self.resolve(underlying, None)?.identity);
            }
            EnrichmentKind::UserDefinedValueType { underlying } => {
                encoder.byte(2);
                encoder.identity(self.resolve(underlying, None)?.identity);
            }
            EnrichmentKind::Struct { fields } => {
                encoder.byte(3);
                let resolved = self.resolve_fields(fields)?;
                for (field, resolved_field) in fields.iter().zip(&resolved) {
                    self.registry
                        .rust_types
                        .entry(field.clone())
                        .or_insert_with(|| resolved_field.field.rust_type.clone());
                }
                encode_fields(&mut encoder, &resolved);
            }
        }
        Ok(encoder.finish())
    }
}

fn fields_identity(tag: u8, fields: &[ResolvedField]) -> OwnedTypeIdentity {
    let mut encoder = IdentityEncoder::new(tag);
    encode_fields(&mut encoder, fields);
    encoder.finish()
}

fn encode_fields(encoder: &mut IdentityEncoder, fields: &[ResolvedField]) {
    encoder.usize(fields.len());
    for field in fields {
        encoder.bytes(field.field.abi_name.as_bytes());
        encoder.bytes(field.field.rust_name.as_bytes());
        encoder.identity(field.identity);
    }
}

fn production_short_digest(value: &[u8]) -> [u8; 16] {
    let mut shortened = [0_u8; 16];
    shortened.copy_from_slice(&value[..16]);
    shortened
}

fn all_fields(abi: &ContractAbi) -> impl Iterator<Item = &AbiField> {
    abi.constructor
        .iter()
        .flat_map(|item| item.inputs.iter())
        .chain(
            abi.functions
                .iter()
                .flat_map(|item| item.inputs.iter().chain(item.outputs.iter())),
        )
        .chain(
            abi.events
                .iter()
                .flat_map(|item| item.inputs.iter().map(|field| &field.field)),
        )
        .chain(abi.errors.iter().flat_map(|item| item.inputs.iter()))
}

fn collision() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InconsistentArtifact,
        "distinct owned tuple identities collide in the generated Rust namespace",
    )
}
