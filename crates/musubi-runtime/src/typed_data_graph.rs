//! Validated and indexed EIP-712 named-type graphs with memoized type hashes.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::{Hash, TypedDataStruct, TypedDataType, keccak256};

use crate::TypedDataError;

pub(crate) const MAX_DEPTH: usize = 64;
pub(crate) const MAX_FIXED_ARRAY_ELEMENTS: usize = 65_536;

/// One validated EIP-712 named-type graph prepared for repeated hashing.
#[derive(Clone, Debug)]
pub struct TypedDataGraph<'a> {
    graph: BTreeMap<&'a str, &'a TypedDataStruct>,
    type_hashes: BTreeMap<&'a str, Hash>,
    maximum_nesting: usize,
    maximum_fixed_array_elements: usize,
}

impl<'a> TypedDataGraph<'a> {
    /// Validates and indexes a complete named-type graph once.
    ///
    /// # Errors
    ///
    /// Returns [`TypedDataError`] for duplicate, unresolved, cyclic, or malformed types.
    pub fn new(types: &'a [TypedDataStruct]) -> Result<Self, TypedDataError> {
        let (graph, maximum_nesting, maximum_fixed_array_elements) = validate_graph(types)?;
        Ok(Self {
            graph,
            type_hashes: BTreeMap::new(),
            maximum_nesting,
            maximum_fixed_array_elements,
        })
    }

    /// Returns the longest combined named-struct and array nesting path.
    #[must_use]
    pub fn maximum_nesting(&self) -> usize {
        self.maximum_nesting
    }

    /// Returns the largest cumulative fixed-array element product along a type path.
    #[must_use]
    pub fn maximum_fixed_array_elements(&self) -> usize {
        self.maximum_fixed_array_elements
    }

    /// Returns one named descriptor from the validated graph.
    ///
    /// # Errors
    ///
    /// Returns [`TypedDataError::InvalidGraph`] when `name` is unknown.
    pub fn descriptor(&self, name: &str) -> Result<&'a TypedDataStruct, TypedDataError> {
        self.graph
            .get(name)
            .copied()
            .ok_or(TypedDataError::InvalidGraph)
    }

    /// Produces canonical EIP-712 `encodeType` text without revalidating the graph.
    ///
    /// # Errors
    ///
    /// Returns [`TypedDataError`] when the primary type is unknown.
    pub fn type_string(&self, primary: &str) -> Result<String, TypedDataError> {
        self.type_string_with_limit(primary, usize::MAX)
    }

    /// Produces canonical EIP-712 `encodeType` text within an explicit byte limit.
    ///
    /// # Errors
    ///
    /// Returns [`TypedDataError`] when the primary type is unknown or the result exceeds
    /// `maximum_bytes` while it is being constructed.
    pub fn type_string_with_limit(
        &self,
        primary: &str,
        maximum_bytes: usize,
    ) -> Result<String, TypedDataError> {
        crate::typed_data_type_text::build(&self.graph, primary, maximum_bytes)
    }

    /// Computes and memoizes one canonical EIP-712 type hash.
    ///
    /// # Errors
    ///
    /// Returns [`TypedDataError`] when the primary type is unknown.
    pub fn type_hash(&mut self, primary: &str) -> Result<Hash, TypedDataError> {
        if let Some(hash) = self.type_hashes.get(primary) {
            return Ok(*hash);
        }
        let name = *self
            .graph
            .get_key_value(primary)
            .ok_or(TypedDataError::InvalidGraph)?
            .0;
        let hash = keccak256(self.type_string(primary)?.as_bytes());
        self.type_hashes.insert(name, hash);
        Ok(hash)
    }

    #[cfg(test)]
    pub(crate) fn memoized_type_count(&self) -> usize {
        self.type_hashes.len()
    }
}

fn validate_graph(
    types: &[TypedDataStruct],
) -> Result<(BTreeMap<&str, &TypedDataStruct>, usize, usize), TypedDataError> {
    let mut graph = BTreeMap::new();
    for ty in types {
        if !identifier(&ty.name)
            || ty.fields.is_empty()
            || graph.insert(ty.name.as_str(), ty).is_some()
        {
            return Err(TypedDataError::InvalidGraph);
        }
        let mut fields = BTreeSet::new();
        if ty
            .fields
            .iter()
            .any(|field| !identifier(&field.name) || !fields.insert(field.name.as_str()))
        {
            return Err(TypedDataError::InvalidGraph);
        }
    }
    for ty in types {
        for field in &ty.fields {
            validate_field_type(&field.ty, &graph, 0)?;
        }
    }
    let mut completed = BTreeSet::new();
    for ty in types {
        detect_cycle(
            ty.name.as_str(),
            &graph,
            &mut BTreeSet::new(),
            &mut completed,
            0,
        )?;
    }
    let mut depths = BTreeMap::new();
    let mut maximum_nesting = 0;
    for ty in types {
        maximum_nesting =
            maximum_nesting.max(struct_depth(ty.name.as_str(), &graph, &mut depths, 0)?);
    }
    let mut products = BTreeMap::new();
    let mut maximum_fixed_array_elements = 1;
    for ty in types {
        maximum_fixed_array_elements = maximum_fixed_array_elements.max(struct_fixed_elements(
            ty.name.as_str(),
            &graph,
            &mut products,
        )?);
    }
    Ok((graph, maximum_nesting, maximum_fixed_array_elements))
}

fn struct_fixed_elements<'a>(
    name: &'a str,
    graph: &BTreeMap<&'a str, &'a TypedDataStruct>,
    products: &mut BTreeMap<&'a str, usize>,
) -> Result<usize, TypedDataError> {
    if let Some(product) = products.get(name) {
        return Ok(*product);
    }
    let descriptor = graph.get(name).ok_or(TypedDataError::InvalidGraph)?;
    let mut maximum = 1;
    for field in &descriptor.fields {
        maximum = maximum.max(type_fixed_elements(&field.ty, graph, products)?);
    }
    products.insert(name, maximum);
    Ok(maximum)
}

fn type_fixed_elements<'a>(
    ty: &'a TypedDataType,
    graph: &BTreeMap<&'a str, &'a TypedDataStruct>,
    products: &mut BTreeMap<&'a str, usize>,
) -> Result<usize, TypedDataError> {
    match ty {
        TypedDataType::Struct(name) => struct_fixed_elements(name, graph, products),
        TypedDataType::Array(element) => type_fixed_elements(element, graph, products),
        TypedDataType::FixedArray(element, length) => {
            type_fixed_elements(element, graph, products)?
                .checked_mul(*length)
                .filter(|product| *product <= MAX_FIXED_ARRAY_ELEMENTS)
                .ok_or(TypedDataError::InvalidGraph)
        }
        _ => Ok(1),
    }
}

fn struct_depth<'a>(
    name: &'a str,
    graph: &BTreeMap<&'a str, &'a TypedDataStruct>,
    depths: &mut BTreeMap<&'a str, usize>,
    traversal_depth: usize,
) -> Result<usize, TypedDataError> {
    if traversal_depth > MAX_DEPTH {
        return Err(TypedDataError::Nesting);
    }
    if let Some(depth) = depths.get(name) {
        return Ok(*depth);
    }
    let descriptor = graph.get(name).ok_or(TypedDataError::InvalidGraph)?;
    let mut maximum = 0;
    for field in &descriptor.fields {
        maximum = maximum.max(type_depth(&field.ty, graph, depths, traversal_depth)?);
    }
    if maximum > MAX_DEPTH {
        return Err(TypedDataError::Nesting);
    }
    depths.insert(name, maximum);
    Ok(maximum)
}

fn type_depth<'a>(
    ty: &'a TypedDataType,
    graph: &BTreeMap<&'a str, &'a TypedDataStruct>,
    depths: &mut BTreeMap<&'a str, usize>,
    traversal_depth: usize,
) -> Result<usize, TypedDataError> {
    let child = match ty {
        TypedDataType::Struct(name) => struct_depth(
            name,
            graph,
            depths,
            traversal_depth
                .checked_add(1)
                .ok_or(TypedDataError::Nesting)?,
        )?,
        TypedDataType::Array(element) | TypedDataType::FixedArray(element, _) => type_depth(
            element,
            graph,
            depths,
            traversal_depth
                .checked_add(1)
                .ok_or(TypedDataError::Nesting)?,
        )?,
        _ => return Ok(0),
    };
    child.checked_add(1).ok_or(TypedDataError::Nesting)
}

fn validate_field_type(
    ty: &TypedDataType,
    graph: &BTreeMap<&str, &TypedDataStruct>,
    depth: usize,
) -> Result<(), TypedDataError> {
    if depth > MAX_DEPTH {
        return Err(TypedDataError::Nesting);
    }
    match ty {
        TypedDataType::FixedBytes(size) if !(1..=32).contains(size) => {
            Err(TypedDataError::InvalidGraph)
        }
        TypedDataType::Uint(bits) | TypedDataType::Int(bits)
            if !(8..=256).contains(bits) || bits % 8 != 0 =>
        {
            Err(TypedDataError::InvalidGraph)
        }
        TypedDataType::Struct(name) if !graph.contains_key(name.as_str()) => {
            Err(TypedDataError::InvalidGraph)
        }
        TypedDataType::FixedArray(_, length)
            if *length == 0 || *length > MAX_FIXED_ARRAY_ELEMENTS =>
        {
            Err(TypedDataError::InvalidGraph)
        }
        TypedDataType::Array(element) | TypedDataType::FixedArray(element, _) => {
            validate_field_type(element, graph, depth + 1)
        }
        _ => Ok(()),
    }
}

fn detect_cycle<'a>(
    name: &'a str,
    graph: &BTreeMap<&'a str, &'a TypedDataStruct>,
    visiting: &mut BTreeSet<&'a str>,
    completed: &mut BTreeSet<&'a str>,
    depth: usize,
) -> Result<(), TypedDataError> {
    if completed.contains(name) {
        return Ok(());
    }
    if depth > MAX_DEPTH || !visiting.insert(name) {
        return Err(TypedDataError::InvalidGraph);
    }
    for field in &graph.get(name).ok_or(TypedDataError::InvalidGraph)?.fields {
        let mut references = Vec::new();
        named_references(&field.ty, &mut references);
        for reference in references {
            detect_cycle(reference, graph, visiting, completed, depth + 1)?;
        }
    }
    visiting.remove(name);
    completed.insert(name);
    Ok(())
}

fn named_references<'a>(ty: &'a TypedDataType, output: &mut Vec<&'a str>) {
    match ty {
        TypedDataType::Struct(name) => output.push(name),
        TypedDataType::Array(element) | TypedDataType::FixedArray(element, _) => {
            named_references(element, output);
        }
        _ => {}
    }
}

fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte == b'$' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte == b'$' || byte.is_ascii_alphanumeric())
}
