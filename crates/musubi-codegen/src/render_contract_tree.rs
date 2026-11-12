//! Incremental semantic owner rendering into the bounded generated tree.

use std::path::Path;

use musubi_core::ArtifactKind;

use crate::{
    ContractIr, GeneratedTree, RenderError, names::rust_pascal, render_abi,
    render_artifact::render_artifact, render_constructor::render_constructor,
    render_contract::contract_source, render_contract_layout::ContractLayout,
    render_operation::operation, render_owned_tuple::render_owned_tuple,
    render_source::BoundedSource, render_type_imports as imports, render_types::TypeRenderer,
};

pub(crate) fn contract_tree(
    tree: &mut GeneratedTree,
    contract: &ContractIr,
) -> Result<(), RenderError> {
    let layout = ContractLayout::new(contract)?;
    let base = Path::new("src").join(&contract.module);
    let source = contract_source(contract, &layout, tree.remaining_bytes())?;
    tree.insert(base.join("mod.rs"), source)?;
    let source = layout.types(contract, tree.remaining_bytes())?;
    tree.insert(base.join("types.rs"), source)?;
    let types = TypeRenderer::new(contract);
    for (leaf, function) in layout.operations.iter().zip(&contract.abi.functions) {
        let mut source = leaf_source(
            tree,
            "One function's owned arguments, returns, codecs, descriptor, and operations.",
        )?;
        source.push(&imports::fields_source(
            &contract.owned_types,
            function.inputs.iter().chain(&function.outputs),
            !function.inputs.is_empty() || !function.outputs.is_empty(),
        ))?;
        types.render_function(&mut source, function)?;
        render_abi::function(&mut source, function)?;
        operation(&mut source, contract, function)?;
        tree.insert(base.join(&leaf.path), source.finish())?;
    }
    for (leaf, event) in layout.events.iter().zip(&contract.abi.events) {
        let mut source = leaf_source(
            tree,
            "One event's owned value, descriptor, and strict log codec.",
        )?;
        source.push(&imports::event_source(&contract.owned_types, event))?;
        types.render_event(&mut source, event)?;
        render_abi::event(&mut source, event)?;
        tree.insert(base.join(&leaf.path), source.finish())?;
    }
    for (leaf, error) in layout.errors.iter().zip(&contract.abi.errors) {
        let mut source = leaf_source(
            tree,
            "One custom error's owned value, conversions, and descriptor.",
        )?;
        source.push(&imports::fields_source(
            &contract.owned_types,
            &error.inputs,
            true,
        ))?;
        types.render_error(&mut source, error)?;
        render_abi::error(&mut source, error)?;
        tree.insert(base.join(&leaf.path), source.finish())?;
    }
    for (leaf, enrichment) in layout.enrichments.iter().zip(&contract.enrichments) {
        let mut source = leaf_source(
            tree,
            "One validated AST enrichment and its ABI-authoritative conversions.",
        )?;
        source.push(&imports::enrichment_source(
            &contract.owned_types,
            enrichment,
        ))?;
        types.render_enrichment(&mut source, enrichment)?;
        tree.insert(base.join(&leaf.path), source.finish())?;
    }
    for (leaf, tuple) in layout.tuples.iter().zip(contract.owned_types.tuples()) {
        let mut source = leaf_source(
            tree,
            "One deduplicated owned tuple surface and its ABI conversions.",
        )?;
        source.push(&imports::tuple_source(tuple))?;
        render_owned_tuple(&mut source, tuple)?;
        tree.insert(base.join(&leaf.path), source.finish())?;
    }
    if contract.kind == ArtifactKind::Deployable {
        constructor(tree, contract, &types, &base)?;
    }
    let mut source = leaf_source(
        tree,
        "Embedded artifact bytes and aggregate input/provenance identity.",
    )?;
    source.push("use musubi_core::ExternalArtifactCoherence;\n")?;
    if contract.kind == ArtifactKind::Deployable {
        source.push("use musubi_core::{Bytes, ContractArtifact};\n")?;
    }
    render_artifact(&mut source, contract)?;
    tree.insert(base.join("artifact.rs"), source.finish())
}

fn constructor(
    tree: &mut GeneratedTree,
    contract: &ContractIr,
    types: &TypeRenderer<'_>,
    base: &Path,
) -> Result<(), RenderError> {
    let mut source = leaf_source(
        tree,
        "Constructor arguments, descriptor, linking, and deployment operations.",
    )?;
    if let Some(constructor) = &contract.abi.constructor {
        // Bytes is already imported by the deployment surface; use its fully qualified spelling there.
        source.push(&imports::fields_source(
            &contract.owned_types,
            &constructor.inputs,
            true,
        ))?;
        types.render_struct(
            &mut source,
            "ConstructorArgs",
            &constructor.inputs,
            "Typed constructor arguments",
        )?;
        TypeRenderer::render_abi_value_methods(
            &mut source,
            "ConstructorArgs",
            "constructor_inputs()",
        )?;
    }
    let marker = rust_pascal(&contract.id);
    source.formatted(format_args!("use super::{{{marker}, artifact}};\n"))?;
    render_abi::constructor(&mut source, &contract.abi)?;
    render_constructor(&mut source, contract, &marker)?;
    tree.insert(base.join("constructor.rs"), source.finish())
}

fn leaf_source(tree: &GeneratedTree, responsibility: &str) -> Result<BoundedSource, RenderError> {
    let mut source = BoundedSource::new(tree.remaining_bytes());
    source.formatted(format_args!("//! {responsibility}\n\n"))?;
    Ok(source)
}
