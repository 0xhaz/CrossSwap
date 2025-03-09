use ark_ff::{BigInteger, Field, PrimeField};
use ark_std::One;
use crate::libraries::types::U256;

/// A trait for computing bit properties of field elements, used in zk-proofs.
pub trait BitMathField<F: Field> {
    fn most_significant_bit(&self, x: F) -> Option<u8>;
    fn least_significant_bit(&self, x: F) -> Option<u8>;
}

/// BitMath module for computing bit properties of unsigned integers, used in Uniswap V4 for tick indexing
/// and price range calculations, and adaptable for zk-proof circuits via field arithmetic.
pub struct BitMath;

impl BitMath {
    /// Returns the index of the most significant bit of a 128-bit number (0 = LSB, 127 = MSB).
    /// Returns None if x == 0.
    pub fn most_significant_bit(x: u128) -> Option<u8> {
        if x == 0 {
            None
        } else {
            Some(127 - x.leading_zeros() as u8)
        }
    }

    /// Returns the index of the most significant bit of a 256-bit number (0 = LSB, 255 = MSB).
    /// Returns None if x == 0.
    pub fn most_significant_bit_u256(x: U256) -> Option<u32> {
        let words = x.0; // Access the four u64 words of U256

        // Check each word from highest to lowest (MSB to LSB)
        for i in (0..4).rev() {
            let word = words[i];
            if word > 0 {
                // Find the most significant bit in this word
                let mut msb = 63; // 63 is the highest bit in a u64
                let mut mask = 1u64 << msb;

                while msb > 0 && (word & mask) == 0 {
                    msb -= 1;
                    mask >>= 1;
                }

                // Return the absolute bit position in the 256-bit U256
                return Some((i as u32 * 64) + msb);
            }
        }

        // Return None if the value is zero
        None
    }

    /// Returns the index of the least significant bit of a 128-bit number (0 = LSB, 127 = MSB).
    /// Returns None if x == 0.
    pub fn least_significant_bit(x: u128) -> Option<u8> {
        if x == 0 {
            None
        } else {
            Some(x.trailing_zeros() as u8)
        }
    }
}

// Constrain F to PrimeField to access into_bigint()
impl<F: PrimeField> BitMathField<F> for BitMath {
    fn most_significant_bit(&self, x: F) -> Option<u8> {
        if x.is_zero() {
            return None;
        }
        // Convert field element to a native integer (e.g., u128) for bit operations
        let x_bigint = x.into_bigint();
        let x_bytes = x_bigint.to_bytes_be();  // Use up to 32 bytes for 254-bit field

        // Construct u128 from the last 16 bytes (128 bits) of the byte representation
        let x_u128 = if x_bytes.len() >= 16 {
            u128::from_be_bytes(
                x_bytes[x_bytes.len() - 16..]
                    .try_into()
                    .unwrap_or_else(|_| [0u8; 16]),  // Default to 0 if too short
            )
        } else {
            // Pad with zeros if less than 16 bytes
            let mut padded = [0u8; 16];
            let offset = 16 - x_bytes.len();
            padded[offset..].copy_from_slice(&x_bytes);
            u128::from_be_bytes(padded)
        };

        BitMath::most_significant_bit(x_u128)
    }

    fn least_significant_bit(&self, x: F) -> Option<u8> {
        if x.is_zero() {
            return None;
        }
        let x_bigint = x.into_bigint();
        let x_bytes = x_bigint.to_bytes_be();  // Use up to 32 bytes for 254-bit field

        // Construct u128 from the last 16 bytes (128 bits) of the byte representation
        let x_u128 = if x_bytes.len() >= 16 {
            u128::from_be_bytes(
                x_bytes[x_bytes.len() - 16..]
                    .try_into()
                    .unwrap_or_else(|_| [0u8; 16]),  // Default to 0 if too short
            )
        } else {
            // Pad with zeros if less than 16 bytes
            let mut padded = [0u8; 16];
            let offset = 16 - x_bytes.len();
            padded[offset..].copy_from_slice(&x_bytes);
            u128::from_be_bytes(padded)
        };

        BitMath::least_significant_bit(x_u128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;  // Example field; adjust for your field type

    #[test]
    fn test_most_significant_bit() {
        assert_eq!(BitMath::most_significant_bit(1), Some(0));
        assert_eq!(BitMath::most_significant_bit(2), Some(1));
        assert_eq!(BitMath::most_significant_bit(4), Some(2));
        assert_eq!(BitMath::most_significant_bit(1 << 64), Some(64));
        assert_eq!(BitMath::most_significant_bit(1 << 127), Some(127));
        assert_eq!(BitMath::most_significant_bit(0), None);
    }

    #[test]
    fn test_least_significant_bit() {
        assert_eq!(BitMath::least_significant_bit(1), Some(0));
        assert_eq!(BitMath::least_significant_bit(2), Some(1));
        assert_eq!(BitMath::least_significant_bit(4), Some(2));
        assert_eq!(BitMath::least_significant_bit(1 << 64), Some(64));
        assert_eq!(BitMath::least_significant_bit(0b1010000), Some(4));
        assert_eq!(BitMath::least_significant_bit(0), None);
    }

    #[test]
    fn test_most_significant_bit_max() {
        assert_eq!(BitMath::most_significant_bit(u128::MAX), Some(127));
    }

    #[test]
    fn test_least_significant_bit_sparse() {
        assert_eq!(BitMath::least_significant_bit(1 << 63), Some(63));
    }

    #[test]
    fn test_most_significant_bit_zero() {
        assert_eq!(BitMath::most_significant_bit(0), None);
    }

    #[test]
    fn test_least_significant_bit_zero() {
        assert_eq!(BitMath::least_significant_bit(0), None);
    }

    #[test]
    fn test_field_most_significant_bit_zero() {
        let zero = <Fr as Field>::ZERO;  // Use ZERO constant from Field trait
        assert_eq!(BitMath.most_significant_bit(zero), None);
    }

    #[test]
    fn test_field_least_significant_bit_zero() {
        let zero = <Fr as Field>::ZERO;  // Use ZERO constant from Field trait
        assert_eq!(BitMath.least_significant_bit(zero), None);
    }

    #[test]
    fn test_field_most_significant_bit_non_zero() {
        // Test with a non-zero field element
        let one = Fr::one();  // Use One trait for `one()`
        let msb = BitMath.most_significant_bit(one);
        assert!(msb.is_some(), "Non-zero field element should have a valid MSB");
        assert_eq!(msb, Some(0));  // For Fr::one(), expect MSB to be 0 (since it's 1 in binary)
    }

    #[test]
    fn test_field_least_significant_bit_non_zero() {
        let one = Fr::one();  // Use One trait for `one()`
        let lsb = BitMath.least_significant_bit(one);
        assert!(lsb.is_some(), "Non-zero field element should have a valid LSB");
        assert_eq!(lsb, Some(0));  // For Fr::one(), expect LSB to be 0 (since it's 1 in binary)
    }
}