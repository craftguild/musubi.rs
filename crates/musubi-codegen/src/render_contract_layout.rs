//! Preflight contract owner inventories and public type re-exports without new public namespaces.

use crate::{
    ContractIr, RenderError,
    render_layout::{Leaf, Naming},
    render_names::{error_type_name, event_type_name},
    render_source::BoundedSource,
    render_types::TypeRenderer,
};

pub(crate) struct ContractLayout {
    pub(crate) operations: Vec<Leaf>,
    pub(crate) events: Vec<Leaf>,
    pub(crate) errors: Vec<Leaf>,
    pub(crate) enrichments: Vec<Leaf>,
    pub(crate) tuples: Vec<Leaf>,
}

impl ContractLayout {
    pub(crate) fn new(contract: &ContractIr) -> Result<Self, RenderError> {
        let mut names = Naming::default();
        Ok(Self {
            operations: contract
                .abi
                .functions
                .iter()
                .map(|item| names.owner("operations", &item.name, item.signature.as_bytes()))
                .collect::<Result<_, _>>()?,
            events: contract
                .abi
                .events
                .iter()
                .map(|item| names.owner("events", &item.name, item.signature.as_bytes()))
                .collect::<Result<_, _>>()?,
            errors: contract
                .abi
                .errors
                .iter()
                .map(|item| names.owner("errors", &item.name, item.signature.as_bytes()))
                .collect::<Result<_, _>>()?,
            enrichments: contract
                .enrichments
                .iter()
                .map(|item| {
                    names.owner(
                        "shared_types",
                        &item.rust_name,
                        item.internal_type.as_bytes(),
                    )
                })
                .collect::<Result<_, _>>()?,
            tuples: contract
                .owned_types
                .tuples()
                .map(|item| names.owner("shared_types", &item.rust_name, &item.identity.0))
                .collect::<Result<_, _>>()?,
        })
    }

    pub(crate) fn modules(&self, source: &mut BoundedSource) -> Result<(), RenderError> {
        for leaf in self
            .operations
            .iter()
            .chain(&self.events)
            .chain(&self.errors)
            .chain(&self.enrichments)
            .chain(&self.tuples)
        {
            source.formatted(format_args!(
                "#[path = {:?}]\nmod {};\n",
                leaf.path, leaf.module
            ))?;
        }
        Ok(())
    }

    pub(crate) fn types(
        &self,
        contract: &ContractIr,
        maximum: usize,
    ) -> Result<String, RenderError> {
        let mut source = BoundedSource::new(maximum);
        source.push(
            "//! Public owned types re-exported from their semantic owner definitions.\n\n",
        )?;
        let types = TypeRenderer::new(contract);
        for (leaf, function) in self.operations.iter().zip(&contract.abi.functions) {
            for name in [types.argument_type(function), types.result_type(function)] {
                if name != "()" {
                    export(&mut source, leaf, &name)?;
                }
            }
        }
        for (leaf, event) in self.events.iter().zip(&contract.abi.events) {
            export(&mut source, leaf, &event_type_name(&contract.abi, event))?;
        }
        for (leaf, error) in self.errors.iter().zip(&contract.abi.errors) {
            export(&mut source, leaf, &error_type_name(&contract.abi, error))?;
        }
        for (leaf, enrichment) in self.enrichments.iter().zip(&contract.enrichments) {
            export(&mut source, leaf, &enrichment.rust_name)?;
        }
        for (leaf, tuple) in self.tuples.iter().zip(contract.owned_types.tuples()) {
            export(&mut source, leaf, &tuple.rust_name)?;
        }
        if contract.abi.constructor.is_some() {
            source.push("pub use super::__evm_constructor::ConstructorArgs;\n")?;
        }
        Ok(source.finish())
    }
}

fn export(source: &mut BoundedSource, leaf: &Leaf, name: &str) -> Result<(), RenderError> {
    source.formatted(format_args!("pub use super::{}::{name};\n", leaf.module))
}
