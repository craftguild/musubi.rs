//! Synthetic hashing fixture for the real-CLI semantic-owner resource test; no compiler claim.
#![forbid(unsafe_code)]

use musubi_core::keccak256;

fn main() {
    let fields = (0..140)
        .map(|index| format!("uint256 field{index}"))
        .collect::<Vec<_>>();
    let type_string = format!("LargeTypedData({})", fields.join(","));
    let type_hash = keccak256(type_string.as_bytes());
    let mut encoded = type_hash.as_ref().to_vec();
    encoded.extend(std::iter::repeat(0).take(32 * 140));
    println!("{}", hex::encode(type_hash.as_ref()));
    println!("{}", hex::encode(keccak256(&encoded).as_ref()));
}
