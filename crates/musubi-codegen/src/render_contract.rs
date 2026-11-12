//! Public contract facade, marker/client state, and genuinely aggregate event/error accessors.

use crate::{
    ContractIr, RenderError, names::rust_pascal, render_contract_layout::ContractLayout,
    render_names::error_type_name, render_source::BoundedSource, render_types::TypeRenderer,
};
use musubi_core::ArtifactKind;

pub(crate) fn contract_source(
    contract: &ContractIr,
    layout: &ContractLayout,
    maximum_bytes: usize,
) -> Result<String, RenderError> {
    let marker = rust_pascal(&contract.id);
    let types = TypeRenderer::new(contract);
    let mut source = BoundedSource::new(maximum_bytes);
    source.push("//! Contract facade, address selection, clients, and aggregate event/revert accessors.\n\npub mod types;\n#[path = \"artifact.rs\"]\nmod __evm_artifact;\npub use __evm_artifact::*;\n\n")?;
    layout.modules(&mut source)?;
    if contract.kind == ArtifactKind::Deployable {
        source.push("#[path = \"constructor.rs\"]\nmod __evm_constructor;\n")?;
    }
    if types.has_public_types() {
        source.push("pub use types::*;\n")?;
    }
    source.push("use musubi_core::{Address, Bytes};\n\n")?;
    source.formatted(format_args!("/// Marker for manifest contract `{}`.\n#[derive(Clone, Copy, Debug, Default)]\npub struct {marker};\n\n", contract.id))?;
    source.formatted(format_args!("/// Address-selected `{marker}` without a backend.\n#[derive(Clone, Copy, Debug)]\npub struct {marker}At {{ address: Address }}\n\n"))?;
    if !contract.abi.functions.is_empty() {
        source.formatted(format_args!("/// Backend-connected `{marker}` client.\n#[derive(Clone, Debug)]\npub struct {marker}Client<B> {{ address: Address, backend: B }}\n\n"))?;
    }
    source.formatted(format_args!("impl {marker} {{\n    /// Selects a deployed contract address.\n    #[must_use]\n    pub fn at(address: Address) -> {marker}At {{ {marker}At {{ address }} }}\n"))?;
    source.push("    /// Returns ABI-authoritative event descriptors.\n    #[must_use]\n    pub fn events() -> Vec<musubi_core::EventDescriptor> { vec![")?;
    for leaf in &layout.events {
        source.formatted(format_args!("{}::descriptor(),", leaf.module))?;
    }
    source.push("] }\n    /// Returns ABI-authoritative custom error descriptors.\n    #[must_use]\n    pub fn errors() -> Vec<musubi_core::ErrorDescriptor> { vec![")?;
    for leaf in &layout.errors {
        source.formatted(format_args!("{}::descriptor(),", leaf.module))?;
    }
    source.push("] }\n}\n\n")?;
    if contract.abi.functions.is_empty() {
        source.formatted(format_args!("impl {marker}At {{\n    /// Returns the selected contract address.\n    #[must_use]\n    pub const fn address(&self) -> Address {{ self.address }}\n}}\n\n"))?;
    } else {
        source.formatted(format_args!("impl {marker}At {{\n    /// Connects a cheaply cloneable backend handle.\n    #[must_use]\n    pub fn connect<B>(self, backend: B) -> {marker}Client<B> {{ {marker}Client {{ address: self.address, backend }} }}\n}}\n\nimpl<B: Clone> {marker}Client<B> {{\n    /// Returns the selected contract address.\n    #[must_use]\n    pub const fn address(&self) -> Address {{ self.address }}\n}}\n\n"))?;
    }
    render_error_surface(&mut source, contract, &marker)?;
    Ok(source.finish())
}

fn render_error_surface(
    source: &mut BoundedSource,
    contract: &ContractIr,
    marker: &str,
) -> Result<(), RenderError> {
    let enum_name = format!("{marker}Error");
    source.formatted(format_args!("/// Typed standard and contract-specific revert union.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub enum {enum_name} {{\n    /// Standard `Error(string)` revert.\n    Error(String),\n    /// Standard `Panic(uint256)` revert.\n    Panic([u8; 32]),\n"))?;
    for error in &contract.abi.errors {
        let error_name = error_type_name(&contract.abi, error);
        source.formatted(format_args!(
            "    /// Custom error `{}`.\n    {error_name}({error_name}),\n",
            error.signature
        ))?;
    }
    source.push("    /// Unrecognized revert bytes retained without interpretation.\n    Unknown(Bytes),\n}\n\n")?;
    source.formatted(format_args!("impl {marker} {{\n    /// Decodes a standard or ABI-declared custom revert.\n    pub fn decode_error(data: &[u8]) -> Result<{enum_name}, musubi_runtime::CodecError> {{\n"))?;
    if !contract.abi.errors.is_empty() {
        source.push("        use musubi_runtime::FromAbiValue as _;\n")?;
    }
    source.formatted(format_args!("        match musubi_runtime::decode_revert(data, &Self::errors())? {{\n            musubi_runtime::DecodedRevert::Error(value) => Ok({enum_name}::Error(value)),\n            musubi_runtime::DecodedRevert::Panic(value) => Ok({enum_name}::Panic(value)),\n"))?;
    if contract.abi.errors.is_empty() {
        source.formatted(format_args!("            musubi_runtime::DecodedRevert::Custom {{ .. }} => Ok({enum_name}::Unknown(Bytes::new(data.to_vec()))),\n"))?;
    } else {
        source.push("            musubi_runtime::DecodedRevert::Custom { signature, values } => match signature.as_str() {\n")?;
        for error in &contract.abi.errors {
            let error_name = error_type_name(&contract.abi, error);
            source.formatted(format_args!("                {:?} => Ok({enum_name}::{error_name}({error_name}::from_abi_value(musubi_core::AbiValue::Tuple(values))?)),\n", error.signature))?;
        }
        source.formatted(format_args!("                _ => Ok({enum_name}::Unknown(Bytes::new(data.to_vec()))),\n            }},\n"))?;
    }
    source.formatted(format_args!("            musubi_runtime::DecodedRevert::Unknown => Ok({enum_name}::Unknown(Bytes::new(data.to_vec()))),\n        }}\n    }}\n}}\n\n"))
}
