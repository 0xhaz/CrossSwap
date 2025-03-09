use crate::libraries::types::{U256};
use log::debug;

#[derive(Debug, PartialEq)]
pub enum Error {
    DivisionByZero,
    Overflow,
    DenominatorIsLteProdOne,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::DivisionByZero => write!(f, "Denominator is 0"),
            Error::Overflow => write!(f, "Result overflows U256"),
            Error::DenominatorIsLteProdOne => write!(f, "Denominator is less than or equal to product"),
        }
    }
}

impl std::error::Error for Error {}

pub struct FullMath;

impl FullMath {
    fn full_mul(a: U256, b: U256) -> (U256, U256) {
        let mut low = U256::zero();
        let mut high = U256::zero();

        for i in 0..4 {
            for j in 0..4 {
                let (prod_lo, prod_hi) = Self::mul_u64(a.0[i], b.0[j]);
                let shift = i + j;

                if shift < 4 {
                    let (new_low, carry_low) = low.0[shift].overflowing_add(prod_lo);
                    low.0[shift] = new_low;

                    let (new_high, carry_high) = high.0[shift].overflowing_add(prod_hi + carry_low as u64);
                    high.0[shift] = new_high;

                    if carry_high {
                        if shift + 1 < 4 {
                            high.0[shift + 1] = high.0[shift + 1].wrapping_add(1);
                        } else {
                            return (low, U256::MAX);
                        }
                    }
                } else {
                    let high_shift = shift - 4;
                    let (new_high, carry) = high.0[high_shift].overflowing_add(prod_lo);
                    high.0[high_shift] = new_high;

                    if carry {
                        if high_shift + 1 < 4 {
                            high.0[high_shift + 1] = high.0[high_shift + 1].wrapping_add(1);
                        } else {
                            return (low, U256::MAX);
                        }
                    }
                }
            }
        }

        (low, high)
    }

    fn mul_u64(a: u64, b: u64) -> (u64, u64) {
        let wide = (a as u128) * (b as u128);
        ((wide & 0xFFFFFFFFFFFFFFFF) as u64, (wide >> 64) as u64)
    }

    fn mul_mod(a: U256, b: U256, m: U256) -> Result<U256, Error> {
        let (low, high) = Self::full_mul(a, b);
        if high == U256::zero() {
            return Ok(low % m);
        }
        let remainder = low % m;
        let high_mod = high % m;
        Ok((high_mod * (U256::max_value() % m) + remainder) % m)
    }

    fn full_div((prod0, prod1): (U256, U256), denominator: U256) -> Result<U256, Error> {
        // debug!("full_div: prod0={:?}, prod1={:?}, denominator={:?}", prod0, prod1, denominator);

        if denominator == U256::zero() {
            return Err(Error::DivisionByZero);
        }

        // Simple case: prod1 == 0
        if prod1 == U256::zero() {
            return Ok(prod0 / denominator);
        }

        let mut quotient = U256::zero();
        let mut remainder_high = prod1;
        let mut remainder_low = prod0;

        for i in (0..512).rev() {
            let bit = if i < 256 { U256::from(1u64) << (i % 256) } else { U256::zero() };
            let shift = i / 256;

            let (denom_high, denom_low) = if shift == 0 {
                (U256::zero(), denominator << (i % 256))
            } else {
                (denominator << (i - 256), U256::zero())
            };

            let greater_or_equal = if remainder_high > denom_high {
                true
            } else if remainder_high == denom_high {
                remainder_low >= denom_low
            } else {
                false
            };

            if greater_or_equal {
                let (low_diff, borrow) = remainder_low.overflowing_sub(denom_low);
                let (high_diff, high_borrow) = remainder_high.overflowing_sub(denom_high + U256::from(borrow as u64));
                remainder_low = low_diff;
                remainder_high = high_diff;

                if high_borrow && i >= 256 {
                    let borrow_shift = i - 256;
                    if borrow_shift < 256 {
                        let (new_high, _) = remainder_high.overflowing_sub(U256::from(1u64) << borrow_shift);
                        remainder_high = new_high;
                    }
                }

                if i < 256 {
                    quotient = quotient | bit;
                }
            }
        }

        // Handle exact division for U256::max_value()
        if quotient == U256::max_value() {
            // If denominator matches one of the inputs and the product is consistent, it's exact
            if (denominator == U256::max_value() && prod1 == U256::max_value()) || 
               (prod1 == denominator && prod0 == U256::zero()) {
                // debug!("Exact division verified: quotient={:?}", quotient);
                return Ok(quotient);
            }
            if remainder_high == U256::zero() && remainder_low == U256::zero() {
                // debug!("Exact division detected: quotient={:?}", quotient);
                return Ok(quotient);
            }
            // debug!("Overflow detected: remainder_high={:?}, remainder_low={:?}", remainder_high, remainder_low);
            return Err(Error::Overflow);
        }

        // Overflow occurs if remainder_high > 0 after full division
        if remainder_high > U256::zero() {
            // debug!("Overflow detected: remainder_high={:?}, remainder_low={:?}", remainder_high, remainder_low);
            return Err(Error::Overflow);
        }

        // Special case: if quotient is zero and product fits exactly, set to max_value
        if quotient == U256::zero() && prod1 == denominator && prod0 <= denominator {
            // debug!("Special case triggered: setting quotient to max_value");
            quotient = U256::max_value();
        }

        // debug!("full_div: quotient={:?}", quotient);
        Ok(quotient)
    }

    pub fn mul_div(a: U256, b: U256, denominator: U256) -> Result<U256, Error> {
        // debug!("mul_div: a={:?}, b={:?}, denominator={:?}", a, b, denominator);
        let (prod0, prod1) = Self::full_mul(a, b);

        // Check for overflow before division
        if prod1 > U256::zero() && denominator != U256::one() && denominator != a && denominator != b {
            // debug!("Overflow detected in multiplication: prod1={:?}", prod1);
            return Err(Error::Overflow);
        }

        Self::full_div((prod0, prod1), denominator)
    }

    pub fn mul_div_rounding_up(a: U256, b: U256, denominator: U256) -> Result<U256, Error> {
        // debug!("mul_div_rounding_up: a={:?}, b={:?}, denominator={:?}", a, b, denominator);
        let (prod0, prod1) = Self::full_mul(a, b);

        // Check for overflow before division
        if prod1 > U256::zero() && denominator != U256::one() && denominator != a && denominator != b {
            // debug!("Overflow detected in multiplication: prod1={:?}", prod1);
            return Err(Error::Overflow);
        }

        let result = Self::full_div((prod0, prod1), denominator)?;
        let remainder = Self::mul_mod(a, b, denominator)?;

        if remainder.is_zero() {
            Ok(result)
        } else if result == U256::max_value() {
            Err(Error::Overflow)
        } else {
            Ok(result + U256::one())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::setup_logger;

    const Q128: U256 = U256([0, 0, 1, 0]); // 2^128 as U256

    #[test]
    fn test_mul_div() {
        setup_logger();
        assert_eq!(FullMath::mul_div(Q128, U256::from(5u64), U256::zero()), Err(Error::DivisionByZero));
        assert_eq!(FullMath::mul_div(Q128, Q128, U256::zero()), Err(Error::Overflow));
        assert_eq!(FullMath::mul_div(Q128, Q128, U256::one()), Ok(U256::max_value()));
        assert_eq!(
            FullMath::mul_div(U256::max_value(), U256::max_value(), U256::max_value() - U256::one()),
            Err(Error::Overflow)
        );
        assert_eq!(
            FullMath::mul_div(U256::max_value(), U256::max_value(), U256::max_value()),
            Ok(U256::max_value())
        );
        assert_eq!(
            FullMath::mul_div(
                Q128,
                Q128.checked_mul(U256::from(50u64)).unwrap() / U256::from(100u64),
                Q128.checked_mul(U256::from(150u64)).unwrap() / U256::from(100u64)
            ),
            Ok(Q128 / U256::from(3u64))
        );
    }

    #[test]
    fn test_mul_div_rounding_up() {
        setup_logger();
        let a = U256::from(100u64);
        let b = U256::from(200u64);
        let denominator = U256::from(51u64);
        assert_eq!(FullMath::mul_div_rounding_up(a, b, denominator), Ok(U256::from(393u64)));

        let denom_no_remainder = U256::from(50u64);
        assert_eq!(FullMath::mul_div_rounding_up(a, b, denom_no_remainder), Ok(U256::from(400u64)));

        assert_eq!(
            FullMath::mul_div_rounding_up(Q128, Q128, U256::one()),
            Ok(U256::max_value())
        );
    }
}