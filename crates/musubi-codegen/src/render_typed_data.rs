//! Owned typed-data struct and hashing API rendering.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use musubi_core::{TypedDataRoot, TypedDataStruct, TypedDataType};

use crate::{
    GeneratedTree, RenderError, TypedDataIr,
    names::{rust_field, rust_pascal, typed_data_digest_method},
    render::render_error,
    render_layout::Naming,
    render_source::BoundedSource,
};

pub(crate) fn typed_data_tree(
    tree: &mut GeneratedTree,
    typed: &TypedDataIr,
) -> Result<(), RenderError> {
    let base = Path::new("src").join(&typed.module);
    let mut names = Naming::default();
    let leaves = typed
        .types
        .iter()
        .map(|descriptor| {
            let identity = format!("{}:{}:{}", typed.id.len(), typed.id, descriptor.name);
            names.owner("types", &descriptor.name, identity.as_bytes())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut source = BoundedSource::new(tree.remaining_bytes());
    source.formatted(format_args!("//! Generated owned EIP-712 types for reviewed authority `{}`.\n\n#[path = \"descriptors.rs\"]\nmod __evm_descriptors;\n", typed.id))?;
    for (leaf, descriptor) in leaves.iter().zip(&typed.types) {
        source.formatted(format_args!(
            "#[path = {:?}]\nmod {};\npub use {}::{};\n",
            leaf.path,
            leaf.module,
            leaf.module,
            rust_pascal(&descriptor.name)
        ))?;
    }
    tree.insert(base.join("mod.rs"), source.finish())?;
    let mut source = BoundedSource::new(tree.remaining_bytes());
    source.push(
        "//! Shared authority type graph; field order preserves the EIP-712 declaration.\n\n",
    )?;
    render_descriptors(&mut source, &typed.types)?;
    tree.insert(base.join("descriptors.rs"), source.finish())?;
    let domains = domain_methods(&typed.roots);
    let mut roots = BTreeMap::<&str, Vec<TypedDataRoot>>::new();
    for root in &typed.roots {
        roots
            .entry(root.primary_type())
            .or_default()
            .push(root.clone());
    }
    let graph = musubi_runtime::TypedDataGraph::new(&typed.types)
        .map_err(|_| render_error("typed-data graph is invalid during rendering"))?;
    for (leaf, descriptor) in leaves.iter().zip(&typed.types) {
        let mut source = BoundedSource::new(tree.remaining_bytes());
        source.formatted(format_args!("//! Owned EIP-712 `{}` value, conversion, and hashing methods.\n\nuse super::__evm_descriptors::descriptors;\n", descriptor.name))?;
        let own_roots = roots
            .get(descriptor.name.as_str())
            .map_or(&[][..], Vec::as_slice);
        let mut dependencies = BTreeSet::new();
        for field in &descriptor.fields {
            type_dependencies(&field.ty, &mut dependencies);
        }
        for domain in own_roots.iter().filter_map(TypedDataRoot::domain_type) {
            dependencies.insert(domain);
        }
        dependencies.remove(descriptor.name.as_str());
        for name in dependencies {
            source.formatted(format_args!("use super::{};\n", rust_pascal(name)))?;
        }
        let type_string = graph
            .type_string_with_limit(&descriptor.name, source.remaining())
            .map_err(|_| render_error("typed-data type string is invalid during rendering"))?;
        let type_hash = musubi_core::keccak256(type_string.as_bytes());
        render_struct(&mut source, descriptor)?;
        render_impl(
            &mut source,
            descriptor,
            own_roots,
            &domains,
            &type_string,
            type_hash,
        )?;
        tree.insert(base.join(&leaf.path), source.finish())?;
    }
    Ok(())
}

fn type_dependencies<'a>(ty: &'a TypedDataType, names: &mut BTreeSet<&'a str>) {
    match ty {
        TypedDataType::Struct(name) => {
            names.insert(name);
        }
        TypedDataType::Array(element) | TypedDataType::FixedArray(element, _) => {
            type_dependencies(element, names);
        }
        _ => {}
    }
}

fn render_struct(
    source: &mut BoundedSource,
    descriptor: &TypedDataStruct,
) -> Result<(), RenderError> {
    let name = rust_pascal(&descriptor.name);
    source.formatted(format_args!(
        "/// Owned EIP-712 `{}` value.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct {name} {{\n",
        descriptor.name
    ))?;
    for (index, field) in descriptor.fields.iter().enumerate() {
        source.formatted(format_args!(
            "    /// Solidity field `{}`.\n    pub {}: {},\n",
            field.name,
            rust_field(&field.name, index),
            rust_type(&field.ty)
        ))?;
    }
    source.push("}\n\n")
}

fn render_impl(
    source: &mut BoundedSource,
    descriptor: &TypedDataStruct,
    roots: &[TypedDataRoot],
    domains: &BTreeMap<&str, usize>,
    type_string: &str,
    type_hash: musubi_core::Hash,
) -> Result<(), RenderError> {
    let name = rust_pascal(&descriptor.name);
    let type_hash = type_hash.into_array();
    source.formatted(format_args!(
        "impl {name} {{\n    /// Canonical EIP-712 encodeType string.\n    pub const TYPE_STRING: &'static str = {:?};\n    /// Returns the canonical EIP-712 type hash.\n    #[must_use]\n    pub const fn type_hash() -> musubi_core::Hash {{ musubi_core::FixedBytes::new({type_hash:?}) }}\n    /// Computes this value's strict EIP-712 struct hash.\n    pub fn struct_hash(&self) -> Result<musubi_core::Hash, musubi_runtime::TypedDataError> {{\n        musubi_runtime::typed_data_struct_hash(&descriptors(), {:?}, &self.typed_data_value())\n    }}\n",
        type_string, descriptor.name
    ))?;
    if domains.contains_key(descriptor.name.as_str()) {
        source.push("    /// Computes this domain value's strict EIP-712 separator.\n    pub fn domain_separator(&self) -> Result<musubi_core::Hash, musubi_runtime::TypedDataError> { Self::struct_hash(self) }\n")?;
    }
    for root in roots
        .iter()
        .filter(|root| root.primary_type() == descriptor.name && root.domain_type().is_some())
    {
        let Some(domain) = root.domain_type() else {
            continue;
        };
        let method = typed_data_digest_method(root, roots)
            .ok_or_else(|| render_error("typed-data root method is invalid during rendering"))?;
        source.formatted(format_args!(
            "    /// Computes the final domain-bound EIP-712 digest.\n    pub fn {method}(&self, domain: &{}) -> Result<musubi_core::Hash, musubi_runtime::TypedDataError> {{\n        musubi_runtime::typed_data_digest(&descriptors(), {:?}, &domain.typed_data_value(), {:?}, &self.typed_data_value())\n    }}\n",
            rust_pascal(domain), domain, descriptor.name
        ))?;
    }
    source.push("    pub(super) fn typed_data_value(&self) -> musubi_core::AbiValue {\n        musubi_core::AbiValue::Tuple(vec![\n")?;
    for (index, field) in descriptor.fields.iter().enumerate() {
        let field_name = rust_field(&field.name, index);
        source.formatted(format_args!(
            "            {},\n",
            value_expression(&field.ty, &format!("self.{field_name}"), false, 0)
        ))?;
    }
    source.push("        ])\n    }\n}\n\n")
}

fn render_descriptors(
    source: &mut BoundedSource,
    types: &[TypedDataStruct],
) -> Result<(), RenderError> {
    source.push("pub(super) fn descriptors() -> Vec<musubi_core::TypedDataStruct> { vec![")?;
    for (type_index, descriptor) in types.iter().enumerate() {
        if type_index != 0 {
            source.push(", ")?;
        }
        source.formatted(format_args!(
            "musubi_core::TypedDataStruct {{ name: {:?}.into(), fields: vec![",
            descriptor.name
        ))?;
        for (field_index, field) in descriptor.fields.iter().enumerate() {
            if field_index != 0 {
                source.push(", ")?;
            }
            source.formatted(format_args!(
                "musubi_core::TypedDataField {{ name: {:?}.into(), ty: ",
                field.name
            ))?;
            render_type_expression(source, &field.ty)?;
            source.push(" }")?;
        }
        source.push("] }")?;
    }
    source.push("] }\n\n")
}

fn render_type_expression(
    source: &mut BoundedSource,
    ty: &TypedDataType,
) -> Result<(), RenderError> {
    match ty {
        TypedDataType::Address => source.push("musubi_core::TypedDataType::Address"),
        TypedDataType::Bool => source.push("musubi_core::TypedDataType::Bool"),
        TypedDataType::String => source.push("musubi_core::TypedDataType::String"),
        TypedDataType::Bytes => source.push("musubi_core::TypedDataType::Bytes"),
        TypedDataType::FixedBytes(size) => source.formatted(format_args!(
            "musubi_core::TypedDataType::FixedBytes({size})"
        )),
        TypedDataType::Uint(bits) => {
            source.formatted(format_args!("musubi_core::TypedDataType::Uint({bits})"))
        }
        TypedDataType::Int(bits) => {
            source.formatted(format_args!("musubi_core::TypedDataType::Int({bits})"))
        }
        TypedDataType::Struct(name) => source.formatted(format_args!(
            "musubi_core::TypedDataType::Struct({name:?}.into())"
        )),
        TypedDataType::Array(element) => {
            source.push("musubi_core::TypedDataType::Array(Box::new(")?;
            render_type_expression(source, element)?;
            source.push("))")
        }
        TypedDataType::FixedArray(element, length) => {
            source.push("musubi_core::TypedDataType::FixedArray(Box::new(")?;
            render_type_expression(source, element)?;
            source.formatted(format_args!("), {length})"))
        }
    }
}

fn rust_type(ty: &TypedDataType) -> String {
    match ty {
        TypedDataType::Address => "musubi_core::Address".into(),
        TypedDataType::Bool => "bool".into(),
        TypedDataType::String => "String".into(),
        TypedDataType::Bytes => "musubi_core::Bytes".into(),
        TypedDataType::FixedBytes(size) => format!("musubi_core::FixedBytes<{size}>"),
        TypedDataType::Uint(bits) => format!("musubi_core::Uint<{bits}>"),
        TypedDataType::Int(bits) => format!("musubi_core::Int<{bits}>"),
        TypedDataType::Struct(name) => rust_pascal(name),
        TypedDataType::Array(element) => format!("Vec<{}>", rust_type(element)),
        TypedDataType::FixedArray(element, length) => format!("[{}; {length}]", rust_type(element)),
    }
}

fn value_expression(ty: &TypedDataType, value: &str, is_reference: bool, depth: usize) -> String {
    match ty {
        TypedDataType::Address => format!(
            "musubi_core::AbiValue::Address({}{value})",
            if is_reference { "*" } else { "" }
        ),
        TypedDataType::Bool => format!(
            "musubi_core::AbiValue::Bool({}{value})",
            if is_reference { "*" } else { "" }
        ),
        TypedDataType::String => format!("musubi_core::AbiValue::String({value}.clone())"),
        TypedDataType::Bytes => format!("musubi_core::AbiValue::Bytes({value}.clone())"),
        TypedDataType::FixedBytes(_) => format!(
            "musubi_core::AbiValue::FixedBytes(musubi_core::Bytes::new({value}.as_ref().to_vec()))"
        ),
        TypedDataType::Uint(_) => format!("musubi_core::AbiValue::Uint({value}.to_be_bytes())"),
        TypedDataType::Int(_) => format!("musubi_core::AbiValue::Int({value}.to_be_bytes())"),
        TypedDataType::Struct(_) => format!("{value}.typed_data_value()"),
        TypedDataType::Array(element) | TypedDataType::FixedArray(element, _) => {
            let item = format!("item_{depth}");
            format!(
                "musubi_core::AbiValue::Array({value}.iter().map(|{item}| {}).collect())",
                value_expression(element, &item, true, depth + 1)
            )
        }
    }
}

fn domain_methods(roots: &[TypedDataRoot]) -> BTreeMap<&str, usize> {
    let mut output = BTreeMap::new();
    for domain in roots.iter().filter_map(TypedDataRoot::domain_type) {
        *output.entry(domain).or_insert(0) += 1;
    }
    output
}
