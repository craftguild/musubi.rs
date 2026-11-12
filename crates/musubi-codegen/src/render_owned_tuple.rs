//! Rendering for pre-resolved anonymous tuple declarations.

use crate::{RenderError, owned_identity::OwnedTuple, render_source::BoundedSource};

/// Renders one anonymous tuple without retaining or traversing an ABI subtree.
pub(crate) fn render_owned_tuple(
    output: &mut BoundedSource,
    tuple: &OwnedTuple,
) -> Result<(), RenderError> {
    let name = &tuple.rust_name;
    output.formatted(format_args!(
        "/// Anonymous ABI tuple.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct {name} {{\n"
    ))?;
    for (index, field) in tuple.fields.iter().enumerate() {
        let display = if field.abi_name.is_empty() {
            format!("#{index}")
        } else {
            field.abi_name.clone()
        };
        output.formatted(format_args!(
            "    /// ABI field `{display}`.\n    pub {}: {},\n",
            field.rust_name, field.rust_type
        ))?;
    }
    output.push("}\n\n")?;
    output.formatted(format_args!(
        "impl IntoAbiValue for {name} {{\n    fn into_abi_value(self) -> AbiValue {{ AbiValue::Tuple(vec![\n"
    ))?;
    for field in &tuple.fields {
        output.formatted(format_args!(
            "        self.{}.into_abi_value(),\n",
            field.rust_name
        ))?;
    }
    output.push("    ]) }\n}\n")?;
    output.formatted(format_args!(
        "impl FromAbiValue for {name} {{\n    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {{\n        let AbiValue::Tuple(values) = value else {{ return Err(CodecError::TypeMismatch); }};\n"
    ))?;
    if tuple.fields.is_empty() {
        output.formatted(format_args!(
            "        if !values.is_empty() {{ return Err(CodecError::Arity); }}\n        Ok({name} {{}})\n"
        ))?;
        return output.push("    }\n}\n\n");
    }
    output.formatted(format_args!(
        "        if values.len() != {} {{ return Err(CodecError::Arity); }}\n        let mut values = values.into_iter();\n        Ok({name} {{\n",
        tuple.fields.len()
    ))?;
    for field in &tuple.fields {
        output.formatted(format_args!(
            "            {}: <{} as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,\n",
            field.rust_name, field.rust_type
        ))?;
    }
    output.push("        })\n    }\n}\n\n")
}
