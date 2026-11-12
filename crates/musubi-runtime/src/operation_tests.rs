//! Function operation descriptor integrity tests.

use musubi_core::{Address, FunctionDescriptor, StateMutability, U256, keccak256};

use crate::{CodecError, FunctionCall, FunctionTransaction};

fn valid_descriptor() -> FunctionDescriptor {
    descriptor(StateMutability::Nonpayable)
}

fn descriptor(state_mutability: StateMutability) -> FunctionDescriptor {
    let signature = "ping()".to_owned();
    let selector = keccak256(signature.as_bytes()).as_ref()[..4]
        .try_into()
        .unwrap();
    FunctionDescriptor {
        name: "ping".into(),
        signature,
        selector,
        inputs: Vec::new(),
        outputs: Vec::new(),
        state_mutability,
    }
}

fn target() -> Address {
    Address::new([0x11; 20])
}

#[test]
fn call_request_accepts_only_read_mutability() {
    for state_mutability in [StateMutability::View, StateMutability::Pure] {
        assert!(
            FunctionCall::<(), ()>::new(descriptor(state_mutability), ())
                .at(target())
                .call_request()
                .is_ok()
        );
    }
    for state_mutability in [StateMutability::Nonpayable, StateMutability::Payable] {
        assert_eq!(
            FunctionCall::<(), ()>::new(descriptor(state_mutability), ()).call_request(),
            Err(CodecError::OperationKind)
        );
    }
}

#[test]
fn transaction_request_accepts_only_state_changing_mutability() {
    for state_mutability in [StateMutability::Nonpayable, StateMutability::Payable] {
        assert!(
            FunctionTransaction::new(descriptor(state_mutability), ())
                .at(target())
                .transaction_request()
                .is_ok()
        );
    }
    for state_mutability in [StateMutability::View, StateMutability::Pure] {
        assert_eq!(
            FunctionTransaction::new(descriptor(state_mutability), ()).transaction_request(),
            Err(CodecError::OperationKind)
        );
    }
}

#[test]
fn function_requests_require_a_target_for_every_supported_mutability() {
    for state_mutability in [StateMutability::View, StateMutability::Pure] {
        assert_eq!(
            FunctionCall::<(), ()>::new(descriptor(state_mutability), ()).call_request(),
            Err(CodecError::MissingTarget)
        );
    }
    for state_mutability in [StateMutability::Nonpayable, StateMutability::Payable] {
        assert_eq!(
            FunctionTransaction::new(descriptor(state_mutability), ()).transaction_request(),
            Err(CodecError::MissingTarget)
        );
    }
}

#[test]
fn transaction_request_rejects_value_for_nonpayable_function() {
    let one = U256::from_u128(1).unwrap();
    assert_eq!(
        FunctionTransaction::new(descriptor(StateMutability::Nonpayable), ())
            .at(target())
            .value(one)
            .transaction_request(),
        Err(CodecError::UnexpectedValue)
    );
}

#[test]
fn transaction_request_accepts_value_for_payable_function() {
    let one = U256::from_u128(1).unwrap();
    let request = FunctionTransaction::new(descriptor(StateMutability::Payable), ())
        .at(target())
        .value(one)
        .transaction_request()
        .unwrap();
    assert_eq!(request.value, one);
}

#[test]
fn call_request_rejects_forged_function_descriptors() {
    for descriptor in forged_descriptors() {
        assert_eq!(
            FunctionCall::<(), ()>::new(descriptor, ()).call_request(),
            Err(CodecError::Selector)
        );
    }
}

#[test]
fn transaction_request_rejects_forged_function_descriptors() {
    for descriptor in forged_descriptors() {
        assert_eq!(
            FunctionTransaction::new(descriptor, ()).transaction_request(),
            Err(CodecError::Selector)
        );
    }
}

fn forged_descriptors() -> [FunctionDescriptor; 2] {
    let mut signature = valid_descriptor();
    signature.name = "other".into();
    let mut selector = valid_descriptor();
    selector.selector = [0; 4];
    [signature, selector]
}
