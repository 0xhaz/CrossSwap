use crate::libraries::types::{U256};

/// Math functions that do not check inputs or outputs.
///
/// Contains methods that perform common math functions but do not do any overflow or underflow checks.
/// Division by zero returns 0 and should be checked externally.
pub struct UnsafeMath;

impl UnsafeMath {
    /// Returns ceil(x / y).
    ///
    /// Division by 0 will return 0, and should be checked externally.
    ///
    /// # Arguments
    /// * `x` - The dividend
    /// * `y` - The divisor
    ///
    /// # Returns
    /// The quotient, ceil(x / y)
    pub fn div_rounding_up(x: U256, y: U256) -> U256 {
        
            if y == U256::zero() {
                return U256::zero();
            }
            let div = x / y;
            let rem = x % y;
            div + if rem > U256::zero() { U256::one() } else { U256::zero() }
        
    }

    /// Calculates floor(a * b / denominator).
    ///
    /// Division by 0 will return 0, and should be checked externally.
    ///
    /// # Arguments
    /// * `a` - The multiplicand
    /// * `b` - The multiplier
    /// * `denominator` - The divisor
    ///
    /// # Returns
    /// The 256-bit result, floor(a * b / denominator)
    pub fn simple_mul_div(a: U256, b: U256, denominator: U256) -> U256 {
        
            if denominator == U256::zero() {
                return U256::zero();
            }
            (a * b) / denominator
        
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_rounding_up() {
        assert_eq!(UnsafeMath::div_rounding_up(U256::from(10u64), U256::from(3u64)), U256::from(4u64)); // ceil(10 / 3) = 4
        assert_eq!(UnsafeMath::div_rounding_up(U256::from(9u64), U256::from(3u64)), U256::from(3u64));  // ceil(9 / 3) = 3
        assert_eq!(UnsafeMath::div_rounding_up(U256::from(0u64), U256::from(5u64)), U256::from(0u64));  // ceil(0 / 5) = 0
        assert_eq!(UnsafeMath::div_rounding_up(U256::from(5u64), U256::from(0u64)), U256::from(0u64));  // Division by 0 returns 0
    }

    #[test]
    fn test_simple_mul_div() {
        assert_eq!(UnsafeMath::simple_mul_div(U256::from(10u64), U256::from(5u64), U256::from(2u64)), U256::from(25u64)); // floor(10 * 5 / 2) = 25
        assert_eq!(UnsafeMath::simple_mul_div(U256::from(9u64), U256::from(3u64), U256::from(4u64)), U256::from(6u64));  // floor(9 * 3 / 4) = 6
        assert_eq!(UnsafeMath::simple_mul_div(U256::from(0u64), U256::from(5u64), U256::from(2u64)), U256::from(0u64)); // floor(0 * 5 / 2) = 0
        assert_eq!(UnsafeMath::simple_mul_div(U256::from(5u64), U256::from(5u64), U256::from(0u64)), U256::from(0u64)); // Division by 0 returns 0
    }
}