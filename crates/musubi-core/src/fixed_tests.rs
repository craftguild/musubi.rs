//! Fixed-point parameter boundary tests.

use crate::{Fixed, Int, UFixed, Uint, ValueError};

#[test]
fn zero_decimal_fixed_values_are_valid() {
    let unsigned = Uint::<8>::from_u128(7).unwrap();
    assert_eq!(
        UFixed::<8, 0>::from_scaled(unsigned).unwrap().into_scaled(),
        unsigned
    );
    let signed = Int::<256>::from_i128(-7).unwrap();
    assert_eq!(
        Fixed::<256, 0>::from_scaled(signed).unwrap().into_scaled(),
        signed
    );
    assert_eq!(
        UFixed::<8, 81>::from_scaled(unsigned),
        Err(ValueError::InvalidDecimals)
    );
}
