//! Typed offline and backend-bound function operation builders.

use std::marker::PhantomData;

use musubi_core::{
    AbiValue, Address, BlockId, CallRequest, FunctionDescriptor, Receipt, StateMutability,
    TransactionRequest, U256,
};

use crate::{
    BackendError, CodecError, IntoAbiValue, ReadBackend, TransactionExecutor,
    codec::encode_function, decode_fields, descriptor::validate_function_descriptor,
};

/// Conversion implemented by generated argument structs.
pub trait EncodeArguments {
    /// Converts an owned argument struct into descriptor-ordered dynamic values.
    fn into_values(self) -> Vec<AbiValue>;
}

impl EncodeArguments for () {
    fn into_values(self) -> Vec<AbiValue> {
        Vec::new()
    }
}

impl<T: IntoAbiValue> EncodeArguments for T {
    fn into_values(self) -> Vec<AbiValue> {
        match self.into_abi_value() {
            AbiValue::Tuple(values) => values,
            value => vec![value],
        }
    }
}

/// Conversion implemented by generated return structs.
pub trait DecodeReturns: Sized {
    /// Builds a typed return value from descriptor-ordered dynamic values.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] if arity or a dynamic variant is not the generated shape.
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError>;
}

impl DecodeReturns for () {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.is_empty() {
            Ok(())
        } else {
            Err(CodecError::Arity)
        }
    }
}

/// Offline typed read operation.
#[derive(Clone, Debug)]
pub struct FunctionCall<A, R> {
    descriptor: FunctionDescriptor,
    arguments: A,
    address: Option<Address>,
    result: PhantomData<fn() -> R>,
}

impl<A, R> FunctionCall<A, R> {
    /// Creates an unbound operation from generated ABI data.
    #[must_use]
    pub fn new(descriptor: FunctionDescriptor, arguments: A) -> Self {
        Self {
            descriptor,
            arguments,
            address: None,
            result: PhantomData,
        }
    }

    /// Selects the target contract address.
    #[must_use]
    pub fn at(mut self, address: Address) -> Self {
        self.address = Some(address);
        self
    }

    /// Binds the operation to a read backend.
    #[must_use]
    pub fn connect<B>(self, backend: B) -> BoundCall<B, A, R> {
        BoundCall {
            operation: self,
            backend,
            block: BlockId::Latest,
        }
    }
}

impl<A: EncodeArguments, R> FunctionCall<A, R> {
    /// Builds an offline backend-neutral call request.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] if the descriptor is forged, is not `view` or `pure`, or generated
    /// argument conversion does not match its ABI descriptor.
    pub fn call_request(self) -> Result<CallRequest, CodecError> {
        validate_function_descriptor(&self.descriptor)?;
        if !matches!(
            self.descriptor.state_mutability,
            StateMutability::View | StateMutability::Pure
        ) {
            return Err(CodecError::OperationKind);
        }
        let to = self.address.ok_or(CodecError::MissingTarget)?;
        let input = encode_function(
            self.descriptor.selector,
            &self.descriptor.inputs,
            &self.arguments.into_values(),
        )?;
        Ok(CallRequest {
            to: Some(to),
            input,
            ..CallRequest::default()
        })
    }
}

/// Backend-bound typed read operation.
#[derive(Clone, Debug)]
pub struct BoundCall<B, A, R> {
    operation: FunctionCall<A, R>,
    backend: B,
    block: BlockId,
}

impl<B, A, R> BoundCall<B, A, R> {
    /// Selects the block used by call and estimate operations.
    #[must_use]
    pub fn block(mut self, block: BlockId) -> Self {
        self.block = block;
        self
    }
}

impl<B, A, R> BoundCall<B, A, R>
where
    B: ReadBackend,
    A: EncodeArguments,
    R: DecodeReturns,
{
    /// Executes the call and decodes its typed return value.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError`] for argument encoding, backend, or return decoding failure.
    pub async fn call(self) -> Result<R, OperationError> {
        let outputs = self.operation.descriptor.outputs.clone();
        let request = self.operation.call_request()?;
        let raw = self.backend.call(request, self.block).await?;
        R::from_values(decode_fields(&outputs, raw.as_ref())?).map_err(Into::into)
    }

    /// Estimates gas for this call-shaped operation.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError`] for argument encoding or backend failure.
    pub async fn estimate(self) -> Result<u64, OperationError> {
        let request = self.operation.call_request()?;
        self.backend
            .estimate_gas(request, self.block)
            .await
            .map_err(Into::into)
    }
}

/// Offline typed state-changing operation.
#[derive(Clone, Debug)]
pub struct FunctionTransaction<A> {
    descriptor: FunctionDescriptor,
    arguments: A,
    address: Option<Address>,
    value: U256,
}

impl<A> FunctionTransaction<A> {
    /// Creates an unbound transaction operation.
    #[must_use]
    pub fn new(descriptor: FunctionDescriptor, arguments: A) -> Self {
        Self {
            descriptor,
            arguments,
            address: None,
            value: U256::ZERO,
        }
    }

    /// Selects the target contract.
    #[must_use]
    pub fn at(mut self, address: Address) -> Self {
        self.address = Some(address);
        self
    }

    /// Selects native value attached to a payable operation.
    #[must_use]
    pub fn value(mut self, value: U256) -> Self {
        self.value = value;
        self
    }

    /// Binds the operation to a transaction executor.
    #[must_use]
    pub fn connect<B>(self, backend: B) -> BoundTransaction<B, A> {
        BoundTransaction {
            operation: self,
            backend,
        }
    }
}

impl<A: EncodeArguments> FunctionTransaction<A> {
    /// Builds an offline unsigned transaction.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] if the descriptor is forged, is not state-changing, receives value
    /// while nonpayable, or generated arguments do not match the ABI descriptor.
    pub fn transaction_request(self) -> Result<TransactionRequest, CodecError> {
        validate_function_descriptor(&self.descriptor)?;
        match self.descriptor.state_mutability {
            StateMutability::View | StateMutability::Pure => {
                return Err(CodecError::OperationKind);
            }
            StateMutability::Nonpayable if self.value != U256::ZERO => {
                return Err(CodecError::UnexpectedValue);
            }
            StateMutability::Nonpayable | StateMutability::Payable => {}
        }
        let to = self.address.ok_or(CodecError::MissingTarget)?;
        let input = encode_function(
            self.descriptor.selector,
            &self.descriptor.inputs,
            &self.arguments.into_values(),
        )?;
        Ok(TransactionRequest {
            to: Some(to),
            input,
            value: self.value,
            ..TransactionRequest::default()
        })
    }
}

/// Executor-bound typed state-changing operation.
#[derive(Clone, Debug)]
pub struct BoundTransaction<B, A> {
    operation: FunctionTransaction<A>,
    backend: B,
}

impl<B, A> BoundTransaction<B, A>
where
    B: TransactionExecutor,
    A: EncodeArguments,
{
    /// Fills, signs, broadcasts, and waits according to executor policy.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError`] for codec, policy, signing, broadcast, or completion failure.
    pub async fn send(self) -> Result<Receipt, OperationError> {
        let request = self.operation.transaction_request()?;
        let filled = self.backend.fill_transaction(request).await?;
        let transaction = self.backend.sign_and_broadcast(filled).await?;
        self.backend
            .wait_for_receipt(transaction)
            .await
            .map_err(Into::into)
    }
}

/// Typed operation failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OperationError {
    /// ABI argument or result mismatch.
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// Backend transport, RPC, or execution failure.
    #[error(transparent)]
    Backend(#[from] BackendError),
}
