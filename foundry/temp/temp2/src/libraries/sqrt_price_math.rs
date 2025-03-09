use crate::libraries::types::{U256, U160, I256};
use crate::libraries::{FullMath, SafeCast, UnsafeMath, FixedPoint};
use crate::libraries::full_math;
use log::debug;

pub struct SqrtPriceMath;

#[derive(Debug, PartialEq)]
pub enum SqrtPriceMathError {
    InvalidPriceOrLiquidity,
    InvalidPrice,
    NotEnoughLiquidity,
    PriceOverflow,
}

impl std::fmt::Display for SqrtPriceMathError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SqrtPriceMathError::InvalidPriceOrLiquidity => write!(f, "InvalidPriceOrLiquidity"),
            SqrtPriceMathError::InvalidPrice => write!(f, "InvalidPrice"),
            SqrtPriceMathError::NotEnoughLiquidity => write!(f, "NotEnoughLiquidity"),
            SqrtPriceMathError::PriceOverflow => write!(f, "PriceOverflow"),
        }
    }
}

impl std::error::Error for SqrtPriceMathError {}

impl From<full_math::Error> for SqrtPriceMathError {
    fn from(err: full_math::Error) -> Self {
        match err {
            full_math::Error::DivisionByZero => SqrtPriceMathError::InvalidPriceOrLiquidity,
            full_math::Error::Overflow => SqrtPriceMathError::PriceOverflow,
            full_math::Error::DenominatorIsLteProdOne => SqrtPriceMathError::NotEnoughLiquidity,
        }
    }
}

impl SqrtPriceMath {
    pub fn get_next_sqrt_price_from_amount0_rounding_up(
        sqrt_p_x96: U160,
        liquidity: u128,
        amount: U256,
        add: bool,
    ) -> Result<U160, SqrtPriceMathError> {
        if amount == U256::zero() {
            return Ok(sqrt_p_x96);
        }
        let numerator1 = U256::from(liquidity) << U256::from(FixedPoint::RESOLUTION_Q96);
        if add {
            let product = amount.checked_mul(U256::from(sqrt_p_x96))
                .ok_or(SqrtPriceMathError::PriceOverflow)?;
            if product / amount == U256::from(sqrt_p_x96) {
                let denominator = numerator1.checked_add(product)
                    .ok_or(SqrtPriceMathError::PriceOverflow)?;
                if denominator >= numerator1 {
                    let result = FullMath::mul_div_rounding_up(numerator1, U256::from(sqrt_p_x96), denominator)
                        .map_err(|_| SqrtPriceMathError::PriceOverflow)?;
                    return SafeCast::to_uint160(result).map_err(|_| SqrtPriceMathError::PriceOverflow);
                }
            }
            let quotient = UnsafeMath::div_rounding_up(numerator1, (numerator1 / U256::from(sqrt_p_x96)) + amount);
            SafeCast::to_uint160(quotient).map_err(|_| SqrtPriceMathError::PriceOverflow)
        } else {
            let product = amount.checked_mul(U256::from(sqrt_p_x96))
                .ok_or(SqrtPriceMathError::PriceOverflow)?;
            if product / amount == U256::from(sqrt_p_x96) && numerator1 > product {
                let denominator = numerator1.checked_sub(product)
                    .ok_or(SqrtPriceMathError::NotEnoughLiquidity)?;
                match FullMath::mul_div_rounding_up(numerator1, U256::from(sqrt_p_x96), denominator) {
                    Ok(result) => SafeCast::to_uint160(result).map_err(|_| SqrtPriceMathError::PriceOverflow),
                    Err(_) => {
                        let adjusted_liquidity = numerator1 / U256::from(sqrt_p_x96);
                        let adjusted_denominator = adjusted_liquidity.checked_sub(amount)
                            .ok_or(SqrtPriceMathError::NotEnoughLiquidity)?;
                        let quotient = UnsafeMath::div_rounding_up(numerator1, adjusted_denominator);
                        SafeCast::to_uint160(quotient).map_err(|_| SqrtPriceMathError::PriceOverflow)
                    }
                }
            } else {
                return Err(SqrtPriceMathError::PriceOverflow);
            }
        }
    }

    pub fn get_next_sqrt_price_from_amount1_rounding_down(
        sqrt_p_x96: U160,
        liquidity: u128,
        amount: U256,
        add: bool,
    ) -> Result<U160, SqrtPriceMathError> {
        if add {
            let quotient = if amount <= U256::from(U160::MAX) {
                (amount << U256::from(FixedPoint::RESOLUTION_Q96)) / U256::from(liquidity)
            } else {
                FullMath::mul_div(amount, FixedPoint::Q96.into(), U256::from(liquidity))
                    .map_err(|_| SqrtPriceMathError::PriceOverflow)?
            };
            SafeCast::to_uint160(U256::from(sqrt_p_x96) + quotient)
                .map_err(|_| SqrtPriceMathError::PriceOverflow)
        } else {
            let numerator = amount.checked_mul(FixedPoint::Q96.into())
                .ok_or(SqrtPriceMathError::PriceOverflow)?;
            let quotient = if numerator <= U256::from(U160::MAX) {
                UnsafeMath::div_rounding_up(numerator, U256::from(liquidity))
            } else {
                FullMath::mul_div_rounding_up(amount, FixedPoint::Q96.into(), U256::from(liquidity))
                    .map_err(|_| SqrtPriceMathError::PriceOverflow)?
            };
            if U256::from(sqrt_p_x96) <= quotient {
                return Err(SqrtPriceMathError::NotEnoughLiquidity);
            }
            let result = U256::from(sqrt_p_x96) - quotient;
            SafeCast::to_uint160(result).map_err(|_| SqrtPriceMathError::PriceOverflow)
        }
    }

    pub fn get_next_sqrt_price_from_input(
        sqrt_p_x96: U160,
        liquidity: u128,
        amount_in: U256,
        zero_for_one: bool,
    ) -> Result<U160, SqrtPriceMathError> {
        if sqrt_p_x96 == U160::zero() || liquidity == 0 {
            return Err(SqrtPriceMathError::InvalidPriceOrLiquidity);
        }

        if zero_for_one {
            Self::get_next_sqrt_price_from_amount0_rounding_up(sqrt_p_x96, liquidity, amount_in, true)
        } else {
            Self::get_next_sqrt_price_from_amount1_rounding_down(sqrt_p_x96, liquidity, amount_in, true)
        }
    }

    pub fn get_next_sqrt_price_from_output(
        sqrt_price_current_x96: U160,
        liquidity: u128,
        amount: U256,
        zero_for_one: bool,
    ) -> Result<U160, SqrtPriceMathError> {
        if sqrt_price_current_x96 == U160::zero() || liquidity == 0 {
            return Err(SqrtPriceMathError::InvalidPriceOrLiquidity);
        }

        if zero_for_one {
            Self::get_next_sqrt_price_from_amount1_rounding_down(sqrt_price_current_x96, liquidity, amount, false)
        } else {
            Self::get_next_sqrt_price_from_amount0_rounding_up(sqrt_price_current_x96, liquidity, amount, false)
        }
    }

    pub fn get_amount0_delta(
        sqrt_price_a_x96: U160,
        sqrt_price_b_x96: U160,
        liquidity: u128,
        round_up: bool,
    ) -> Result<U256, SqrtPriceMathError> {
        let (sqrt_lower, sqrt_upper) = if sqrt_price_a_x96 > sqrt_price_b_x96 {
            (sqrt_price_b_x96, sqrt_price_a_x96)
        } else {
            (sqrt_price_a_x96, sqrt_price_b_x96)
        };
    
        if sqrt_lower == U160::zero() {
            return Err(SqrtPriceMathError::InvalidPrice);
        }
    
        let diff = U256::from(sqrt_upper)
            .checked_sub(U256::from(sqrt_lower))
            .ok_or(SqrtPriceMathError::PriceOverflow)?;
        let liquidity_u256 = U256::from(liquidity);
        let q96 = U256::from(1u128 << FixedPoint::RESOLUTION_Q96);
    
        let numerator = liquidity_u256
            .checked_mul(diff)
            .ok_or(SqrtPriceMathError::PriceOverflow)?
            .checked_mul(q96)
            .ok_or(SqrtPriceMathError::PriceOverflow)?;
    
        let denominator = U256::from(sqrt_lower)
            .checked_mul(U256::from(sqrt_upper))
            .ok_or(SqrtPriceMathError::PriceOverflow)?;
    
        // println!("get_amount0_delta: diff={:?}, liquidity={:?}, numerator={:?}, denominator={:?}", diff, liquidity_u256, numerator, denominator);
    
        let result = if round_up {
            FullMath::mul_div_rounding_up(numerator, U256::one(), denominator)
                .map_err(|_| SqrtPriceMathError::PriceOverflow)?
        } else {
            FullMath::mul_div(numerator, U256::one(), denominator)
                .map_err(|_| SqrtPriceMathError::PriceOverflow)?
        };
    
        // println!("get_amount0_delta: result={:?}", result);
        Ok(result)
    }

    pub fn abs_diff(a: U160, b: U160) -> U256 {
        let a_u256 = U256::from(a);
        let b_u256 = U256::from(b);
        if a_u256 >= b_u256 {
            a_u256 - b_u256
        } else {
            b_u256 - a_u256
        }
    }

    pub fn get_amount1_delta(
        sqrt_price_a_x96: U160,
        sqrt_price_b_x96: U160,
        liquidity: u128,
        round_up: bool,
    ) -> Result<U256, SqrtPriceMathError> {
        let (sqrt_lower, sqrt_upper) = if sqrt_price_a_x96 > sqrt_price_b_x96 {
            (sqrt_price_b_x96, sqrt_price_a_x96)
        } else {
            (sqrt_price_a_x96, sqrt_price_b_x96)
        };
    
        let diff = U256::from(sqrt_upper)
            .checked_sub(U256::from(sqrt_lower))
            .ok_or(SqrtPriceMathError::PriceOverflow)?;
        let liquidity_u256 = U256::from(liquidity);
    
        if diff == U256::zero() {
            return Ok(U256::zero());
        }
    
        let numerator = liquidity_u256
            .checked_mul(diff)
            .ok_or(SqrtPriceMathError::PriceOverflow)?;
        let denominator = U256::from(1u128 << FixedPoint::RESOLUTION_Q96);
    
        // println!("get_amount1_delta: diff={:?}, liquidity={:?}, numerator={:?}, denominator={:?}", diff, liquidity_u256, numerator, denominator);
    
        let result = if round_up {
            FullMath::mul_div_rounding_up(numerator, U256::one(), denominator)
                .map_err(|_| SqrtPriceMathError::PriceOverflow)?
        } else {
            FullMath::mul_div(numerator, U256::one(), denominator)
                .map_err(|_| SqrtPriceMathError::PriceOverflow)?
        };
    
        // println!("get_amount1_delta: result={:?}", result);
        Ok(result)
    }

    pub fn get_amount0_delta_signed(
        sqrt_price_a_x96: U160,
        sqrt_price_b_x96: U160,
        liquidity: i128,
    ) -> Result<I256, SqrtPriceMathError> {
        // println!("get_amount0_delta_signed: sqrt_price_a_x96={:?}, sqrt_price_b_x96={:?}, liquidity={:?}", sqrt_price_a_x96, sqrt_price_b_x96, liquidity);
        if liquidity < 0 {
            let unsigned_liquidity = (-liquidity) as u128;
            let delta = Self::get_amount0_delta(sqrt_price_a_x96, sqrt_price_b_x96, unsigned_liquidity, false)?;
            // println!("get_amount0_delta_signed: delta={:?}, returning {:?}", delta, -I256::from(delta));
            Ok(-I256::from(delta))
        } else {
            let unsigned_liquidity = liquidity as u128;
            let delta = Self::get_amount0_delta(sqrt_price_a_x96, sqrt_price_b_x96, unsigned_liquidity, true)?;
            // println!("get_amount0_delta_signed: delta={:?}, returning {:?}", delta, I256::from(delta));
            Ok(I256::from(delta))
        }
    }

    pub fn get_amount1_delta_signed(
        sqrt_price_a_x96: U160,
        sqrt_price_b_x96: U160,
        liquidity: i128,
    ) -> Result<I256, SqrtPriceMathError> {
        // println!("get_amount1_delta_signed: sqrt_price_a_x96={:?}, sqrt_price_b_x96={:?}, liquidity={:?}", sqrt_price_a_x96, sqrt_price_b_x96, liquidity);
        if liquidity < 0 {
            let unsigned_liquidity = (-liquidity) as u128;
            let delta = Self::get_amount1_delta(sqrt_price_a_x96, sqrt_price_b_x96, unsigned_liquidity, false)?;
            // println!("get_amount1_delta_signed: delta={:?}, returning {:?}", delta, -I256::from(delta));
            Ok(-I256::from(delta))
        } else {
            let unsigned_liquidity = liquidity as u128;
            let delta = Self::get_amount1_delta(sqrt_price_a_x96, sqrt_price_b_x96, unsigned_liquidity, true)?;
            // println!("get_amount1_delta_signed: delta={:?}, returning {:?}", delta, I256::from(delta));
            Ok(I256::from(delta))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_amount0_delta() {
        let q96 = U256::from(1u64) << 96;
        let sqrt_price_a_x96 = U160::from(100u64) * U160::from(q96.as_u128());
        let sqrt_price_b_x96 = U160::from(110u64) * U160::from(q96.as_u128());
        let liquidity = 1_000_000u128;
        let round_up = false;

        let result = SqrtPriceMath::get_amount0_delta(sqrt_price_a_x96, sqrt_price_b_x96, liquidity, round_up);
        assert!(result.is_ok(), "Result should be Ok: {:?}", result);
        let delta = result.unwrap();
        // println!("get_amount0_delta (100*Q96, 110*Q96, 1,000,000, false): {:?}", delta);
        assert!(delta > U256::zero(), "Delta should be greater than zero: {:?}", delta);
        assert_eq!(delta, U256::from(909u64), "Delta should be 909: {:?}", delta);
    }

    #[test]
    fn test_get_amount0_delta_low_liquidity() {
        let q96 = U256::from(1u64) << 96;
        let sqrt_price_a_x96 = U160::from(100u64) * U160::from(q96.as_u128());
        let sqrt_price_b_x96 = U160::from(110u64) * U160::from(q96.as_u128());
        let liquidity = 1000u128;
        let round_up = false;

        let result = SqrtPriceMath::get_amount0_delta(sqrt_price_a_x96, sqrt_price_b_x96, liquidity, round_up);
        assert!(result.is_ok(), "Result should be Ok: {:?}", result);
        let delta = result.unwrap();
        // println!("get_amount0_delta (100*Q96, 110*Q96, 1000, false): {:?}", delta);
        assert_eq!(delta, U256::from(0u64), "Delta should be 0 due to truncation: {:?}", delta);
    }

    #[test]
    fn test_get_amount0_delta_increasing_price() {
        let q96 = U256::from(1u64) << 96;
        let sqrt_price_a_x96 = U160::from(100u64) * U160::from(q96.as_u128());
        let sqrt_price_b_x96 = U160::from(110u64) * U160::from(q96.as_u128());
        let liquidity = 1_000_000u128;
        let round_up = false;

        let result = SqrtPriceMath::get_amount0_delta(sqrt_price_a_x96, sqrt_price_b_x96, liquidity, round_up);
        assert!(result.is_ok(), "Result should be Ok: {:?}", result);
        let delta = result.unwrap();
        // println!("get_amount0_delta (100*Q96, 110*Q96, 1,000,000, false): {:?}", delta);
        assert!(delta > U256::zero(), "Delta should be greater than zero: {:?}", delta);
        assert!(
            delta >= U256::from(909u64) && delta <= U256::from(910u64),
            "Delta should be approximately 909: {:?}", delta
        );
    }

    #[test]
    fn test_get_amount1_delta() {
        let q96 = U256::from(1u64) << 96;
        let sqrt_price_a_x96 = U160::from(100u64) * U160::from(q96.as_u128());
        let sqrt_price_b_x96 = U160::from(110u64) * U160::from(q96.as_u128());
        let liquidity = 1_000_000u128;
        let round_up = true;

        let result = SqrtPriceMath::get_amount1_delta(sqrt_price_a_x96, sqrt_price_b_x96, liquidity, round_up);
        assert!(result.is_ok(), "Result should be Ok: {:?}", result);
        let delta = result.unwrap();
        // println!("Delta: {:?}", delta);
        assert!(delta > U256::zero(), "Delta should be greater than zero: {:?}", delta);
        assert_eq!(delta, U256::from(10000000u128), "Delta should be approximately 10,000,000: {:?}", delta);
    }
}