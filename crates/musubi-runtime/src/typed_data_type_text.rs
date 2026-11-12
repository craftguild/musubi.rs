//! Incremental bounded construction of canonical EIP-712 type text.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{TypedDataStruct, TypedDataType};

use crate::{TypedDataError, typed_data_graph::MAX_DEPTH};

pub(crate) fn build(
    graph: &BTreeMap<&str, &TypedDataStruct>,
    primary: &str,
    maximum_bytes: usize,
) -> Result<String, TypedDataError> {
    let primary_type = graph.get(primary).ok_or(TypedDataError::InvalidGraph)?;
    let mut dependencies = BTreeSet::new();
    collect_dependencies(primary_type, graph, &mut dependencies, 0)?;
    dependencies.remove(primary);
    let mut output = BoundedText::new(maximum_bytes);
    definition(&mut output, primary_type)?;
    for dependency in dependencies {
        definition(
            &mut output,
            graph.get(dependency).ok_or(TypedDataError::InvalidGraph)?,
        )?;
    }
    Ok(output.value)
}

fn collect_dependencies<'a>(
    ty: &'a TypedDataStruct,
    graph: &BTreeMap<&'a str, &'a TypedDataStruct>,
    output: &mut BTreeSet<&'a str>,
    depth: usize,
) -> Result<(), TypedDataError> {
    if depth > MAX_DEPTH {
        return Err(TypedDataError::Nesting);
    }
    for field in &ty.fields {
        if let Some(reference) = named_reference(&field.ty)
            && output.insert(reference)
        {
            collect_dependencies(
                graph.get(reference).ok_or(TypedDataError::InvalidGraph)?,
                graph,
                output,
                depth + 1,
            )?;
        }
    }
    Ok(())
}

fn named_reference(ty: &TypedDataType) -> Option<&str> {
    match ty {
        TypedDataType::Struct(name) => Some(name),
        TypedDataType::Array(element) | TypedDataType::FixedArray(element, _) => {
            named_reference(element)
        }
        _ => None,
    }
}

fn definition(output: &mut BoundedText, ty: &TypedDataStruct) -> Result<(), TypedDataError> {
    output.push(&ty.name)?;
    output.push("(")?;
    for (index, field) in ty.fields.iter().enumerate() {
        if index != 0 {
            output.push(",")?;
        }
        render_type(output, &field.ty)?;
        output.push(" ")?;
        output.push(&field.name)?;
    }
    output.push(")")
}

fn render_type(output: &mut BoundedText, ty: &TypedDataType) -> Result<(), TypedDataError> {
    match ty {
        TypedDataType::Address => output.push("address"),
        TypedDataType::Bool => output.push("bool"),
        TypedDataType::String => output.push("string"),
        TypedDataType::Bytes => output.push("bytes"),
        TypedDataType::FixedBytes(size) => output.number("bytes", *size, ""),
        TypedDataType::Uint(bits) => output.number("uint", *bits, ""),
        TypedDataType::Int(bits) => output.number("int", *bits, ""),
        TypedDataType::Struct(name) => output.push(name),
        TypedDataType::Array(element) => {
            render_type(output, element)?;
            output.push("[]")
        }
        TypedDataType::FixedArray(element, size) => {
            render_type(output, element)?;
            output.number("[", *size, "]")
        }
    }
}

struct BoundedText {
    value: String,
    maximum_bytes: usize,
}

impl BoundedText {
    fn new(maximum_bytes: usize) -> Self {
        Self {
            value: String::new(),
            maximum_bytes,
        }
    }

    fn push(&mut self, value: &str) -> Result<(), TypedDataError> {
        if self
            .value
            .len()
            .checked_add(value.len())
            .is_none_or(|length| length > self.maximum_bytes)
        {
            return Err(TypedDataError::ResourceLimit);
        }
        self.value.push_str(value);
        Ok(())
    }

    fn number(&mut self, prefix: &str, value: usize, suffix: &str) -> Result<(), TypedDataError> {
        self.push(prefix)?;
        self.push(&value.to_string())?;
        self.push(suffix)
    }
}
