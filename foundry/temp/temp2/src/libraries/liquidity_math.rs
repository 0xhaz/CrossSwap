
// Define a custom error type to mimic Solidity's revert behavior
#[derive(Debug, PartialEq)]
pub enum LiquidityMathError {
    SafeCastOverflow,
}

impl std::fmt::Display for LiquidityMathError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LiquidityMathError::SafeCastOverflow => write!(f, "SafeCastOverflow"),
        }
    }
}

impl std::error::Error for LiquidityMathError {}

pub struct LiquidityMath;

impl LiquidityMath {
    /// Adds a signed liquidity delta to liquidity and returns the result, or an error if it overflows or underflows.
    /// 
    /// # Arguments
    /// * `x` - The liquidity before change (uint128)
    /// * `y` - The delta by which liquidity should be changed (int128)
    /// 
    /// # Returns
    /// * `Result<u128, LiquidityMathError>` - The resulting liquidity after adding the delta, or an error on overflow/underflow
    pub fn add_delta(x: u128, y: i128) -> Result<u128, LiquidityMathError> {
        // Perform the addition with checked arithmetic, handling signed delta
        match x.checked_add_signed(y) {
            Some(z) => Ok(z),
            None => Err(LiquidityMathError::SafeCastOverflow),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_delta_positive() {
        assert_eq!(LiquidityMath::add_delta(100, 50).unwrap(), 150);
        assert_eq!(LiquidityMath::add_delta(0, 0).unwrap(), 0);
        assert_eq!(LiquidityMath::add_delta(u128::MAX - 1, 1).unwrap(), u128::MAX);
    }

    #[test]
    fn test_add_delta_negative() {
        assert_eq!(LiquidityMath::add_delta(100, -50).unwrap(), 50);
        assert_eq!(LiquidityMath::add_delta(50, -50).unwrap(), 0);
    }

    #[test]
    fn test_add_delta_overflow() {
        assert!(matches!(
            LiquidityMath::add_delta(u128::MAX, 1),
            Err(LiquidityMathError::SafeCastOverflow)
        ));
    }

    #[test]
    fn test_add_delta_underflow() {
        assert!(matches!(
            LiquidityMath::add_delta(0, -1),
            Err(LiquidityMathError::SafeCastOverflow)
        ));
        assert!(matches!(
            LiquidityMath::add_delta(50, -51),
            Err(LiquidityMathError::SafeCastOverflow)
        ));
    }
}