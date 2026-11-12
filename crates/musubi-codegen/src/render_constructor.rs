//! Generated constructor and deployment surface rendering.

use musubi_core::StateMutability;

use crate::{ContractIr, RenderError, render_source::BoundedSource};

pub(crate) fn render_constructor(
    source: &mut BoundedSource,
    contract: &ContractIr,
    marker: &str,
) -> Result<(), RenderError> {
    let (parameter, values, encode_import) = match &contract.abi.constructor {
        Some(constructor) if !constructor.inputs.is_empty() => (
            "arguments: ConstructorArgs, ",
            "arguments.into_values()",
            "        use musubi_runtime::EncodeArguments as _;\n",
        ),
        Some(_) | None => ("", "Vec::new()", ""),
    };
    let value_guard = if matches!(
        contract
            .abi
            .constructor
            .as_ref()
            .map(|value| value.state_mutability),
        Some(StateMutability::Payable)
    ) {
        ""
    } else {
        "        if value != musubi_core::U256::ZERO { return Err(musubi_runtime::CodecError::UnexpectedValue.into()); }\n"
    };
    source.formatted(format_args!("impl {marker} {{\n    /// Builds linked constructor init code with exact library coverage.\n    pub fn init_code({parameter}libraries: &musubi_runtime::LibraryAddresses) -> Result<musubi_core::Bytes, musubi_runtime::DeploymentError> {{\n{encode_import}        musubi_runtime::build_init_code(&artifact().creation, libraries, &constructor_inputs(), &{values})\n    }}\n    /// Builds an unsigned contract-creation transaction after enforcing constructor payability.\n    pub fn deployment({parameter}libraries: &musubi_runtime::LibraryAddresses, value: musubi_core::U256) -> Result<musubi_core::TransactionRequest, musubi_runtime::DeploymentError> {{\n{value_guard}        let input = Self::init_code("))?;
    if parameter.is_empty() {
        source.push("libraries")?;
    } else {
        source.push("arguments, libraries")?;
    }
    source.push(")?; Ok(musubi_core::TransactionRequest { to: None, input, value, ..musubi_core::TransactionRequest::default() })\n    }\n}\n\n")
}
