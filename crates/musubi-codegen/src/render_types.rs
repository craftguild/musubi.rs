//! Generated owned type declarations and dynamic-value conversions.

use musubi_core::{AbiField, AbiType, ErrorDescriptor, EventDescriptor, FunctionDescriptor};

use crate::{
    ContractIr, Enrichment, EnrichmentKind, RenderError,
    names::{rust_field, rust_variant},
    owned_encoding::primitive_rust_type,
    render_names::{error_type_name, event_type_name, function_type_name},
    render_source::BoundedSource,
};

pub(crate) struct TypeRenderer<'a> {
    contract: &'a ContractIr,
}

impl<'a> TypeRenderer<'a> {
    pub(crate) fn new(contract: &'a ContractIr) -> Self {
        Self { contract }
    }

    pub(crate) fn has_public_types(&self) -> bool {
        !self.contract.enrichments.is_empty()
            || self.contract.owned_types.tuples().next().is_some()
            || self
                .contract
                .abi
                .functions
                .iter()
                .any(|function| !function.inputs.is_empty() || !function.outputs.is_empty())
            || !self.contract.abi.events.is_empty()
            || !self.contract.abi.errors.is_empty()
            || self.contract.abi.constructor.is_some()
    }

    pub(crate) fn argument_type(&self, function: &FunctionDescriptor) -> String {
        if function.inputs.is_empty() {
            "()".into()
        } else {
            format!("{}Args", function_type_name(&self.contract.abi, function))
        }
    }

    pub(crate) fn result_type(&self, function: &FunctionDescriptor) -> String {
        if function.outputs.is_empty() {
            "()".into()
        } else {
            format!("{}Return", function_type_name(&self.contract.abi, function))
        }
    }

    pub(crate) fn render_function(
        &self,
        output: &mut BoundedSource,
        function: &FunctionDescriptor,
    ) -> Result<(), RenderError> {
        let base = function_type_name(&self.contract.abi, function);
        if !function.inputs.is_empty() {
            self.render_struct(
                output,
                &format!("{base}Args"),
                &function.inputs,
                &format!("Arguments for `{}`", function.signature),
            )?;
            Self::render_abi_value_methods(
                output,
                &format!("{base}Args"),
                "abi::descriptor().inputs",
            )?;
        }
        if !function.outputs.is_empty() {
            self.render_result(
                output,
                &format!("{base}Return"),
                &function.outputs,
                &function.signature,
            )?;
        }
        Ok(())
    }

    fn render_result(
        &self,
        output: &mut BoundedSource,
        name: &str,
        fields: &[AbiField],
        signature: &str,
    ) -> Result<(), RenderError> {
        self.render_struct(
            output,
            name,
            fields,
            &format!("Return values for `{signature}`"),
        )?;
        output.formatted(format_args!("impl musubi_runtime::DecodeReturns for {name} {{\n    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {{\n"))?;
        self.render_from_values(output, name, fields)?;
        output.push("    }\n}\n\n")?;
        Self::render_abi_value_methods(output, name, "abi::descriptor().outputs")
    }

    pub(crate) fn render_event(
        &self,
        output: &mut BoundedSource,
        event: &EventDescriptor,
    ) -> Result<(), RenderError> {
        let name = event_type_name(&self.contract.abi, event);
        output.formatted(format_args!(
            "/// Typed event `{}`.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct {name} {{\n",
            event.signature
        ))?;
        for (index, event_field) in event.inputs.iter().enumerate() {
            let field = &event_field.field;
            let field_name = field_name(field, index);
            let ty = if event_field.indexed && indexed_is_hash(&field.ty) {
                "Hash".into()
            } else {
                self.rust_type(field)
            };
            output.formatted(format_args!(
                "    /// ABI event field `{}`.\n    pub {field_name}: {ty},\n",
                display_name(field, index)
            ))?;
        }
        output.push("}\n\n")?;
        output.formatted(format_args!("impl {name} {{\n    /// Decodes one log with strict signature, topic-count, and canonical-data checks.\n    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {{\n        let values = musubi_runtime::decode_event(&descriptor(), log)?;\n"))?;
        if event.inputs.is_empty() {
            output.push("        if !values.is_empty() { return Err(CodecError::Arity); }\n")?;
        } else {
            output.push("        let mut values = values.into_iter();\n")?;
        }
        output.push("        Ok(Self {\n")?;
        for (index, event_field) in event.inputs.iter().enumerate() {
            let field = &event_field.field;
            let field_name = field_name(field, index);
            if event_field.indexed && indexed_is_hash(&field.ty) {
                output.formatted(format_args!("            {field_name}: match values.next().ok_or(CodecError::Arity)? {{ musubi_runtime::DecodedEventField::TopicHash(value) => value, _ => return Err(CodecError::TypeMismatch) }},\n"))?;
            } else {
                output.formatted(format_args!("            {field_name}: match values.next().ok_or(CodecError::Arity)? {{ musubi_runtime::DecodedEventField::Value(value) => <{} as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) }},\n", self.rust_type(field)))?;
            }
        }
        output.push("        })\n    }\n    /// Encodes this value as a deterministic metadata-free fixture log.\n    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {\n        let values = vec![\n")?;
        for (index, event_field) in event.inputs.iter().enumerate() {
            let field_name = field_name(&event_field.field, index);
            if event_field.indexed && indexed_is_hash(&event_field.field.ty) {
                output.formatted(format_args!(
                    "            musubi_runtime::DecodedEventField::TopicHash(self.{field_name}),\n"
                ))?;
            } else if self.field_is_copy(&event_field.field) {
                output.formatted(format_args!("            musubi_runtime::DecodedEventField::Value(self.{field_name}.into_abi_value()),\n"))?;
            } else {
                output.formatted(format_args!("            musubi_runtime::DecodedEventField::Value(IntoAbiValue::into_abi_value(&self.{field_name})),\n"))?;
            }
        }
        output.formatted(format_args!("        ];\n        musubi_runtime::encode_event(&descriptor(), address, &values)\n    }}\n}}\n\n"))
    }

    pub(crate) fn render_error(
        &self,
        output: &mut BoundedSource,
        error: &ErrorDescriptor,
    ) -> Result<(), RenderError> {
        let name = error_type_name(&self.contract.abi, error);
        self.render_struct(
            output,
            &name,
            &error.inputs,
            &format!("Typed custom error `{}`", error.signature),
        )
    }

    pub(crate) fn render_enrichment(
        &self,
        output: &mut BoundedSource,
        enrichment: &Enrichment,
    ) -> Result<(), RenderError> {
        let internal_type = rustdoc_text(&enrichment.internal_type);
        match &enrichment.kind {
            EnrichmentKind::Enum { variants, .. } => {
                output.formatted(format_args!("/// AST-enriched Solidity enum `{internal_type}`.\n#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]\npub enum {} {{\n", enrichment.rust_name))?;
                for variant in variants {
                    output.formatted(format_args!(
                        "    /// Solidity enum variant `{variant}`.\n    {},\n",
                        rust_variant(variant)
                    ))?;
                }
                output.push("}\n\n")?;
                Self::render_enum_conversion(output, enrichment, variants)
            }
            EnrichmentKind::UserDefinedValueType { underlying } => {
                let ty = Self::rust_type_base(underlying);
                output.formatted(format_args!("/// Transparent AST-enriched Solidity value type `{internal_type}`.\n#[derive(Clone, Debug, Eq, Hash, PartialEq)]\npub struct {}(pub {ty});\n\n", enrichment.rust_name))?;
                output.formatted(format_args!("impl IntoAbiValue for {} {{ fn into_abi_value(self) -> AbiValue {{ self.0.into_abi_value() }} }}\n", enrichment.rust_name))?;
                output.formatted(format_args!("impl FromAbiValue for {} {{ fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {{ Ok(Self(<{ty} as FromAbiValue>::from_abi_value(value)?)) }} }}\n\n", enrichment.rust_name))
            }
            EnrichmentKind::Struct { fields } => self.render_struct(
                output,
                &enrichment.rust_name,
                fields,
                &format!("AST-enriched Solidity struct `{internal_type}`"),
            ),
        }
    }

    fn render_enum_conversion(
        output: &mut BoundedSource,
        enrichment: &Enrichment,
        variants: &[String],
    ) -> Result<(), RenderError> {
        output.formatted(format_args!("impl IntoAbiValue for {} {{\n    fn into_abi_value(self) -> AbiValue {{\n        let discriminant = match self {{\n", enrichment.rust_name))?;
        for (index, variant) in variants.iter().enumerate() {
            output.formatted(format_args!(
                "            Self::{} => {index}u8,\n",
                rust_variant(variant)
            ))?;
        }
        output.push("        };\n        let mut word = [0u8; 32]; word[31] = discriminant; AbiValue::Uint(word)\n    }\n}\n")?;
        output.formatted(format_args!("impl FromAbiValue for {} {{\n    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {{\n        let AbiValue::Uint(word) = value else {{ return Err(CodecError::TypeMismatch); }};\n        if word[..31].iter().any(|byte| *byte != 0) {{ return Err(CodecError::TypeMismatch); }}\n        match word[31] {{\n", enrichment.rust_name))?;
        for (index, variant) in variants.iter().enumerate() {
            output.formatted(format_args!(
                "            {index} => Ok(Self::{}),\n",
                rust_variant(variant)
            ))?;
        }
        output.push("            _ => Err(CodecError::TypeMismatch),\n        }\n    }\n}\n\n")
    }

    pub(crate) fn render_struct(
        &self,
        output: &mut BoundedSource,
        name: &str,
        fields: &[AbiField],
        documentation: &str,
    ) -> Result<(), RenderError> {
        self.render_plain_struct(output, name, fields, documentation)?;
        output.formatted(format_args!("impl IntoAbiValue for {name} {{\n    fn into_abi_value(self) -> AbiValue {{ AbiValue::Tuple(vec![\n"))?;
        for (index, field) in fields.iter().enumerate() {
            output.formatted(format_args!(
                "        self.{}.into_abi_value(),\n",
                field_name(field, index)
            ))?;
        }
        output.push("    ]) }\n}\n")?;
        output.formatted(format_args!("impl FromAbiValue for {name} {{\n    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {{\n        let AbiValue::Tuple(values) = value else {{ return Err(CodecError::TypeMismatch); }};\n"))?;
        self.render_from_values(output, name, fields)?;
        output.push("    }\n}\n\n")
    }

    fn render_plain_struct(
        &self,
        output: &mut BoundedSource,
        name: &str,
        fields: &[AbiField],
        documentation: &str,
    ) -> Result<(), RenderError> {
        output.formatted(format_args!(
            "/// {documentation}.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct {name} {{\n"
        ))?;
        for (index, field) in fields.iter().enumerate() {
            output.formatted(format_args!(
                "    /// ABI field `{}`.\n    pub {}: {},\n",
                display_name(field, index),
                field_name(field, index),
                self.rust_type(field)
            ))?;
        }
        output.push("}\n\n")
    }

    pub(crate) fn render_abi_value_methods(
        output: &mut BoundedSource,
        name: &str,
        fields: &str,
    ) -> Result<(), RenderError> {
        output.formatted(format_args!(
            "impl {name} {{\n    /// Encodes this value without a function selector using Solidity ABI tuple rules.\n    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {{\n        musubi_runtime::encode_fields(&{fields}, &<Self as musubi_runtime::EncodeArguments>::into_values(self))\n    }}\n    /// Strictly decodes a selector-free Solidity ABI tuple value.\n    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {{\n        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&{fields}, data)?))\n    }}\n}}\n\n"
        ))
    }

    fn render_from_values(
        &self,
        output: &mut BoundedSource,
        name: &str,
        fields: &[AbiField],
    ) -> Result<(), RenderError> {
        if fields.is_empty() {
            return output.formatted(format_args!("        if !values.is_empty() {{ return Err(CodecError::Arity); }}\n        Ok({name} {{}})\n"));
        }
        output.formatted(format_args!("        if values.len() != {} {{ return Err(CodecError::Arity); }}\n        let mut values = values.into_iter();\n        Ok({name} {{\n", fields.len()))?;
        for (index, field) in fields.iter().enumerate() {
            output.formatted(format_args!("            {}: <{} as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,\n", field_name(field, index), self.rust_type(field)))?;
        }
        output.push("        })\n")
    }

    fn rust_type(&self, field: &AbiField) -> String {
        self.contract
            .owned_types
            .rust_type(field)
            .map_or_else(String::new, str::to_owned)
    }

    fn field_is_copy(&self, field: &AbiField) -> bool {
        copy_abi_type(&field.ty) && self.rust_type(field) == Self::rust_type_base(&field.ty)
    }

    fn rust_type_base(ty: &AbiType) -> String {
        match ty {
            AbiType::Array(element) => format!("Vec<{}>", Self::rust_type_base(element)),
            AbiType::FixedArray(element, length) => {
                format!("[{}; {length}]", Self::rust_type_base(element))
            }
            _ => primitive_rust_type(ty),
        }
    }
}

fn copy_abi_type(ty: &AbiType) -> bool {
    match ty {
        AbiType::Address
        | AbiType::Bool
        | AbiType::FixedBytes(_)
        | AbiType::Uint(_)
        | AbiType::Int(_)
        | AbiType::UFixed(_, _)
        | AbiType::Fixed(_, _) => true,
        AbiType::FixedArray(element, _) => copy_abi_type(element),
        AbiType::String
        | AbiType::Bytes
        | AbiType::Function
        | AbiType::Array(_)
        | AbiType::Tuple(_) => false,
    }
}

fn field_name(field: &AbiField, index: usize) -> String {
    rust_field(&field.name, index)
}
fn rustdoc_text(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}
fn display_name(field: &AbiField, index: usize) -> String {
    if field.name.is_empty() {
        format!("#{index}")
    } else {
        field.name.clone()
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
