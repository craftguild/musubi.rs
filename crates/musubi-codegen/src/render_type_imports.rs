//! Exact generated type-module imports derived from validated ABI surfaces.

use std::{collections::BTreeSet, fmt::Write as _};

use musubi_core::{AbiField, AbiType, ContractAbi};

use crate::{
    Enrichment, EnrichmentKind,
    owned_identity::{OwnedTuple, OwnedTypeRegistry},
};

const CORE_TYPES: &[&str] = &[
    "AbiValue",
    "Address",
    "Bytes",
    "Fixed",
    "FixedBytes",
    "FunctionPointer",
    "Hash",
    "Int",
    "UFixed",
    "Uint",
];
const RUNTIME_TYPES: &[&str] = &["CodecError", "FromAbiValue", "IntoAbiValue"];

#[derive(Default)]
struct TypeImports {
    core: BTreeSet<&'static str>,
    runtime: BTreeSet<&'static str>,
    shared: BTreeSet<String>,
}

impl TypeImports {
    fn conversions(&mut self) {
        self.decode();
        self.runtime.insert("IntoAbiValue");
    }

    fn decode(&mut self) {
        self.core.insert("AbiValue");
        self.runtime.extend(["CodecError", "FromAbiValue"]);
    }

    fn field(&mut self, owned_types: &OwnedTypeRegistry, field: &AbiField) {
        if let Some(rust_type) = owned_types.rust_type(field) {
            self.rust_type(rust_type);
        }
    }

    fn rust_type(&mut self, rust_type: &str) {
        for token in rust_type
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        {
            if let Some(candidate) = CORE_TYPES.iter().find(|candidate| **candidate == token) {
                self.core.insert(candidate);
            } else if token
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
                && !["Vec", "String", "bool"].contains(&token)
            {
                self.shared.insert(token.into());
            }
        }
    }

    fn source(&self) -> String {
        let mut source = String::new();
        render_use(&mut source, "musubi_core", CORE_TYPES, &self.core);
        render_use(&mut source, "musubi_runtime", RUNTIME_TYPES, &self.runtime);
        if !self.shared.is_empty() {
            writeln!(
                source,
                "use super::types::{{{}}};",
                self.shared.iter().cloned().collect::<Vec<_>>().join(", ")
            )
            .expect("String formatting cannot fail");
        }
        if !source.is_empty() {
            source.push('\n');
        }
        source
    }
}

pub(crate) fn fields_source(
    owned: &OwnedTypeRegistry,
    fields: impl IntoIterator<Item = impl std::borrow::Borrow<AbiField>>,
    conversions: bool,
) -> String {
    let mut imports = TypeImports::default();
    if conversions {
        imports.conversions();
    }
    for field in fields {
        imports.field(owned, field.borrow());
    }
    imports.source()
}

pub(crate) fn event_source(
    owned: &OwnedTypeRegistry,
    event: &musubi_core::EventDescriptor,
) -> String {
    let mut imports = TypeImports::default();
    collect_event(&mut imports, event, owned);
    imports.source()
}

pub(crate) fn enrichment_source(owned: &OwnedTypeRegistry, enrichment: &Enrichment) -> String {
    let mut imports = TypeImports::default();
    collect_enrichment(&mut imports, enrichment, owned);
    imports.source()
}

pub(crate) fn tuple_source(tuple: &OwnedTuple) -> String {
    let mut imports = TypeImports::default();
    imports.conversions();
    for field in &tuple.fields {
        imports.rust_type(&field.rust_type);
    }
    imports.source()
}

pub(crate) fn type_import_names(
    abi: &ContractAbi,
    enrichments: &[Enrichment],
    owned_types: &OwnedTypeRegistry,
) -> BTreeSet<String> {
    let imports = collect_type_imports(abi, enrichments, owned_types);
    imports
        .core
        .into_iter()
        .chain(imports.runtime)
        .map(str::to_owned)
        .collect()
}

fn collect_type_imports(
    abi: &ContractAbi,
    enrichments: &[Enrichment],
    owned_types: &OwnedTypeRegistry,
) -> TypeImports {
    let mut imports = TypeImports::default();
    for tuple in owned_types.tuples() {
        imports.conversions();
        for field in &tuple.fields {
            imports.rust_type(&field.rust_type);
        }
    }
    for enrichment in enrichments {
        collect_enrichment(&mut imports, enrichment, owned_types);
    }
    for function in &abi.functions {
        if !function.inputs.is_empty() {
            imports.conversions();
            collect_fields(&mut imports, owned_types, &function.inputs);
        }
        if !function.outputs.is_empty() {
            imports.conversions();
            collect_fields(&mut imports, owned_types, &function.outputs);
        }
    }
    for event in &abi.events {
        collect_event(&mut imports, event, owned_types);
    }
    for error in &abi.errors {
        imports.conversions();
        collect_fields(&mut imports, owned_types, &error.inputs);
    }
    if let Some(constructor) = &abi.constructor {
        imports.conversions();
        collect_fields(&mut imports, owned_types, &constructor.inputs);
    }
    imports
}

fn collect_enrichment(
    imports: &mut TypeImports,
    enrichment: &Enrichment,
    owned: &OwnedTypeRegistry,
) {
    imports.conversions();
    match &enrichment.kind {
        EnrichmentKind::Enum { .. } => {}
        EnrichmentKind::UserDefinedValueType { underlying } => {
            collect_abi_type(imports, underlying);
        }
        EnrichmentKind::Struct { fields } => collect_fields(imports, owned, fields),
    }
}

fn collect_event(
    imports: &mut TypeImports,
    event: &musubi_core::EventDescriptor,
    owned_types: &OwnedTypeRegistry,
) {
    imports.runtime.insert("CodecError");
    for event_field in &event.inputs {
        if event_field.indexed && indexed_is_hash(&event_field.field.ty) {
            imports.core.insert("Hash");
        } else {
            imports.runtime.extend(["FromAbiValue", "IntoAbiValue"]);
            imports.field(owned_types, &event_field.field);
        }
    }
}

fn collect_fields(imports: &mut TypeImports, owned_types: &OwnedTypeRegistry, fields: &[AbiField]) {
    for field in fields {
        imports.field(owned_types, field);
    }
}

fn collect_abi_type(imports: &mut TypeImports, ty: &AbiType) {
    match ty {
        AbiType::Array(element) | AbiType::FixedArray(element, _) => {
            collect_abi_type(imports, element);
        }
        AbiType::Tuple(fields) => {
            for field in fields {
                collect_abi_type(imports, &field.ty);
            }
        }
        _ => imports.rust_type(&crate::owned_encoding::primitive_rust_type(ty)),
    }
}

fn indexed_is_hash(ty: &AbiType) -> bool {
    matches!(
        ty,
        AbiType::String
            | AbiType::Bytes
            | AbiType::Array(_)
            | AbiType::FixedArray(_, _)
            | AbiType::Tuple(_)
    )
}

fn render_use(
    source: &mut String,
    module: &str,
    order: &[&'static str],
    selected: &BTreeSet<&'static str>,
) {
    let names = order
        .iter()
        .copied()
        .filter(|candidate| selected.contains(candidate))
        .collect::<Vec<_>>();
    if !names.is_empty() {
        writeln!(source, "use {module}::{{{}}};", names.join(", "))
            .expect("String formatting cannot fail");
    }
}
