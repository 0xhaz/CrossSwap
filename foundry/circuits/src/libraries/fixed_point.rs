use ark_ff::Field;
use crate::libraries::types::{U256};

/// A trait for handling binary fixed-point numbers in field arithmetic, used in zk-proofs.
pub trait FixedPointField<F: Field> {
    fn q96() -> F;
    fn q128() -> F;
}

/// A library for handling binary fixed-point numbers, used in Uniswap V4 math.
pub struct FixedPoint;

impl FixedPoint {
    /// Resolution for Q96 fixed-point format (96 bits of fractional precision).
    pub const RESOLUTION_Q96: u8 = 96;

    /// Base for Q96 fixed-point arithmetic, equivalent to 2^96.
    /// This is a 128-bit integer, but note that Uniswap V4 uses 256 bits in Solidity.
    /// For full precision, consider using `U256` from the `uint` crate.
    pub const Q96: u128 = 0x1000000000000000000000000;

    /// Base for Q128 fixed-point arithmetic, equivalent to 2^128.
    /// This uses U256 for full 256-bit precision, matching Uniswap V4.
    /// 2^128 is represented by setting the third limb (index 2) to 1, since 128 / 64 = 2.
    pub const Q128: U256 = U256([0, 0, 1, 0]);  // Limb 2 (index 2) is set to 1 for 2^128
}

impl<F: Field> FixedPointField<F> for FixedPoint {
    fn q96() -> F {
        // Convert Q96 (u128) to field element
        F::from(FixedPoint::Q96 as u128)  // Simplified; adjust for field modulus if needed
    }

    fn q128() -> F {
        // Convert Q128 (U256) to field element
        // This is a placeholder; implement based on your field (e.g., ark_bn254::Fr)
        unimplemented!("Convert U256 to field element F")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_point() {
        assert_eq!(FixedPoint::RESOLUTION_Q96, 96);
        assert_eq!(FixedPoint::Q96, 0x1000000000000000000000000);
    }

    #[test]
    fn test_fixed_point_q128() {
        let q128 = FixedPoint::Q128;
        assert_eq!(q128, U256([0, 0, 1, 0]));  // 2^128 is 1 in the third limb (index 2)
    }
}