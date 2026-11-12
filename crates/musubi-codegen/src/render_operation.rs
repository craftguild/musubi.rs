//! Offline operation and bound-client methods co-located with one canonical function owner.

use crate::{
    ContractIr, RenderError, names::rust_pascal, render_names::function_method_name,
    render_source::BoundedSource, render_types::TypeRenderer,
};
use musubi_core::{FunctionDescriptor, StateMutability};

pub(crate) fn operation(
    source: &mut BoundedSource,
    contract: &ContractIr,
    function: &FunctionDescriptor,
) -> Result<(), RenderError> {
    let marker = rust_pascal(&contract.id);
    let types = TypeRenderer::new(contract);
    let import = if matches!(
        function.state_mutability,
        StateMutability::View | StateMutability::Pure
    ) {
        "FunctionCall"
    } else {
        "FunctionTransaction"
    };
    source.formatted(format_args!("use super::{{{marker}, {marker}Client}};\nuse musubi_runtime::{import};\n\nimpl {marker} {{\n"))?;
    let method = function_method_name(&contract.abi, function);
    let args = types.argument_type(function);
    let result = types.result_type(function);
    let parameter = (!function.inputs.is_empty()).then(|| format!("arguments: {args}"));
    let value = if function.inputs.is_empty() {
        "()"
    } else {
        "arguments"
    };
    let (operation, constructor) = if matches!(
        function.state_mutability,
        StateMutability::View | StateMutability::Pure
    ) {
        (
            format!("FunctionCall<{args}, {result}>"),
            "FunctionCall::new",
        )
    } else {
        (
            format!("FunctionTransaction<{args}>"),
            "FunctionTransaction::new",
        )
    };
    source.formatted(format_args!("    /// Builds `{}` as an offline typed operation.\n    #[must_use]\n    pub fn {method}({}) -> {operation} {{ {constructor}(abi::descriptor(), {value}) }}\n", function.signature, parameter.as_deref().unwrap_or("")))?;

    source.formatted(format_args!("}}\n\nimpl<B: Clone> {marker}Client<B> {{\n"))?;
    let method = function_method_name(&contract.abi, function);
    let args = types.argument_type(function);
    let result = types.result_type(function);
    let parameter = (!function.inputs.is_empty()).then(|| format!("arguments: {args}"));
    let value = if function.inputs.is_empty() {
        ""
    } else {
        "arguments"
    };
    let bound = if matches!(
        function.state_mutability,
        StateMutability::View | StateMutability::Pure
    ) {
        format!("musubi_runtime::BoundCall<B, {args}, {result}>")
    } else {
        format!("musubi_runtime::BoundTransaction<B, {args}>")
    };
    source.formatted(format_args!("    /// Builds backend-bound `{}`.\n    #[must_use]\n    pub fn {method}(&self, {}) -> {bound} {{ {marker}::{method}({value}).at(self.address).connect(self.backend.clone()) }}\n", function.signature, parameter.as_deref().unwrap_or("")))?;

    source.push("}\n\n")
}
