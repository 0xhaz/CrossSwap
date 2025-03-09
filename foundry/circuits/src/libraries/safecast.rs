use crate::libraries::types::{U256, U160};

/// Safe casting methods.
///
/// Contains methods for safely casting between types, reverting on overflow or underflow.
pub struct SafeCast;

/// Custom error types for SafeCast operations.
#[derive(Debug, PartialEq)]
pub enum SafeCastError {
    Overflow,
}

impl std::fmt::Display for SafeCastError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SafeCastError::Overflow => write!(f, "SafeCastOverflow"),
        }
    }
}

impl std::error::Error for SafeCastError {}

impl SafeCast {
    /// Cast a uint256 to a uint160, revert on overflow.
    ///
    /// # Arguments
    /// * `x` - The uint256 to be downcasted
    ///
    /// # Returns
    /// The downcasted integer, now type U160
    pub fn to_uint160(x: U256) -> Result<U160, SafeCastError> {
        // Create a byte buffer for U256 (32 bytes)
        let mut bytes = [0u8; 32];
        x.to_little_endian(&mut bytes); // Write U256 to bytes
        let y = U160::from_little_endian(&mut bytes[..20]); // Take first 20 bytes (160 bits)
        if extend_u160_to_u256(y) != x {
            return Err(SafeCastError::Overflow);
        }
        Ok(y)
    }

    /// Cast a uint256 to a uint128, revert on overflow.
    ///
    /// # Arguments
    /// * `x` - The uint256 to be downcasted
    ///
    /// # Returns
    /// The downcasted integer, now type u128
    pub fn to_uint128(x: U256) -> Result<u128, SafeCastError> {
        // Check if x fits within u128 (0 to 2^128 - 1)
        let max_u128 = U256::from(u128::MAX);
        if x > max_u128 {
            return Err(SafeCastError::Overflow);
        }
        // Safely convert U256 to u128 using byte manipulation or as_u128
        Ok(x.as_u128())
    }

    /// Cast a int128 to a uint128, revert on overflow or underflow.
    ///
    /// # Arguments
    /// * `x` - The int128 to be casted
    ///
    /// # Returns
    /// The casted integer, now type u128
    pub fn to_uint128_from_i128(x: i128) -> Result<u128, SafeCastError> {
        if x < 0 {
            return Err(SafeCastError::Overflow);
        }
        Ok(x as u128) // Safe since x >= 0
    }

    /// Cast a int256 to a int128, revert on overflow or underflow.
    ///
    /// # Arguments
    /// * `x` - The int256 (U256 interpreted as signed) to be downcasted
    ///
    /// # Returns
    /// The downcasted integer, now type i128
    pub fn to_int128(x: U256) -> Result<i128, SafeCastError> {
        // Interpret U256 as unsigned and check if it fits in i128 (0 to 2^127 - 1, since U256 is unsigned)
        let max_i128 = U256::from(i128::MAX as u128); // 2^127 - 1

        if x > max_i128 {
            return Err(SafeCastError::Overflow);
        }
        // Convert to i128, ensuring no overflow (U256 values are non-negative, so no underflow)
        Ok(x.as_u128() as i128)
    }

    /// Cast a uint256 to a int256, revert on overflow.
    ///
    /// # Arguments
    /// * `x` - The uint256 to be casted
    ///
    /// # Returns
    /// The casted integer, now type int256 (U256 interpreted as signed)
    pub fn to_int256(x: U256) -> Result<U256, SafeCastError> {
        // U256 is already unsigned, but we check if it would overflow as signed (int256 range: -2^255 to 2^255 - 1)
        // For uint256 to int256, we need to ensure x < 2^255 (since int256 max is 2^255 - 1)
        let max_int256 = U256::from(1u64) << 255; // 2^255
        if x >= max_int256 {
            return Err(SafeCastError::Overflow);
        }
        Ok(x) // No change in representation, just checking for overflow
    }

    /// Cast a uint256 to a int128, revert on overflow.
    ///
    /// # Arguments
    /// * `x` - The uint256 to be downcasted
    ///
    /// # Returns
    /// The downcasted integer, now type int128
    pub fn to_int128_from_uint256(x: U256) -> Result<i128, SafeCastError> {
        // Check if x fits within i128 (0 to 2^127 - 1, since uint256 is unsigned)
        let max_i128 = U256::from(i128::MAX as u128); // 2^127 - 1
        if x > max_i128 {
            return Err(SafeCastError::Overflow);
        }
        // Safely convert to i128
        Ok(x.as_u128() as i128)
    }
}

// Helper function to extend U160 to U256 by padding with zeros in the upper 96 bits
fn extend_u160_to_u256(value: U160) -> U256 {
    let mut bytes = [0u8; 32]; // 32 bytes for U256
    let mut value_bytes = [0u8; 24]; // 24 bytes for U160
    value.to_little_endian(&mut value_bytes); // Write U160 to bytes
    bytes[..20].copy_from_slice(&value_bytes[..20]); // Copy 160 bits (20 bytes) to lower part of U256
    U256::from_little_endian(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_uint160() {
        assert_eq!(SafeCast::to_uint160(U256::from(160u64)), Ok(U160::from(160u64)));
        assert!(SafeCast::to_uint160(U256::from(1u64) << 160).is_err()); // Overflow
    }

    #[test]
    fn test_to_uint128() {
        assert_eq!(SafeCast::to_uint128(U256::from(128u64)), Ok(128u128));
        assert!(SafeCast::to_uint128(U256::from(1u64) << 128).is_err()); // Overflow
    }

    #[test]
    fn test_to_uint128_from_i128() {
        assert_eq!(SafeCast::to_uint128_from_i128(128i128), Ok(128u128));
        assert!(SafeCast::to_uint128_from_i128(-1i128).is_err()); // Underflow
    }

    #[test]
    fn test_to_int128() {
        assert_eq!(SafeCast::to_int128(U256::from(127u64)), Ok(127i128));
        assert!(SafeCast::to_int128(U256::from(1u64) << 127).is_err()); // Overflow
    }

    #[test]
    fn test_to_int256() {
        assert_eq!(SafeCast::to_int256(U256::from(255u64)), Ok(U256::from(255u64)));
        assert!(SafeCast::to_int256(U256::from(1u64) << 255).is_err()); // Overflow (negative when interpreted as i256)
    }

    #[test]
    fn test_to_int128_from_uint256() {
        assert_eq!(SafeCast::to_int128_from_uint256(U256::from(127u64)), Ok(127i128));
        assert!(SafeCast::to_int128_from_uint256(U256::from(1u64) << 127).is_err()); // Overflow
    }
}