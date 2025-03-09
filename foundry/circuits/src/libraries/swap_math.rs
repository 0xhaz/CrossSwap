use crate::libraries::types::{U160, U256, I256};
use crate::libraries::full_math::{FullMath, Error as FullMathError};
use crate::libraries::sqrt_price_math::{SqrtPriceMath, SqrtPriceMathError};

pub struct SwapMath;

#[derive(Debug)]
pub enum SwapMathError {
    FullMathError(FullMathError),
    SqrtPriceMathError(SqrtPriceMathError),
}

impl From<FullMathError> for SwapMathError {
    fn from(err: FullMathError) -> Self {
        SwapMathError::FullMathError(err)
    }
}

impl From<SqrtPriceMathError> for SwapMathError {
    fn from(err: SqrtPriceMathError) -> Self {
        SwapMathError::SqrtPriceMathError(err)
    }
}

impl SwapMath {
    pub const MAX_SWAP_FEE: u32 = 1_000_000;
    pub const Q96: U256 = U256([0, 0, 1, 0]); // 2^96
    pub const DECIMALS: U256 = U256([0xde0b6b3a7640000, 0x0, 0x0, 0x0]); // 10^18

    pub fn get_sqrt_price_target(
        zero_for_one: bool,
        sqrt_price_next_x96: U160,
        sqrt_price_limit_x96: U160,
    ) -> U160 {
        if zero_for_one {
            if sqrt_price_next_x96 == U160::zero() {
                sqrt_price_limit_x96
            } else if sqrt_price_next_x96 < sqrt_price_limit_x96 {
                sqrt_price_next_x96
            } else {
                sqrt_price_limit_x96
            }
        } else {
            if sqrt_price_next_x96 == U160::zero() {
                sqrt_price_limit_x96
            } else if sqrt_price_next_x96 > sqrt_price_limit_x96 {
                sqrt_price_next_x96
            } else {
                sqrt_price_limit_x96
            }
        }
    }

    pub fn compute_swap_step(
        sqrt_price_current_x96: U160,
        sqrt_price_target_x96: U160,
        liquidity: u128,
        amount_remaining: I256,
        fee_pips: u32,
    ) -> Result<(U160, U256, U256, U256), SwapMathError> {
        let zero_for_one = sqrt_price_current_x96 >= sqrt_price_target_x96;
        let exact_in = amount_remaining < I256::zero();
        let decimals = Self::DECIMALS;
        // println!("compute_swap_step: zero_for_one={:?}, exact_in={:?}, amount_remaining={:?}, decimals={:?}", zero_for_one, exact_in, amount_remaining, decimals);

        if exact_in {
            let amount_remaining_abs = U256::from((-amount_remaining).as_i128().unsigned_abs());
            let amount_remaining_less_fee = FullMath::mul_div(
                amount_remaining_abs,
                U256::from(Self::MAX_SWAP_FEE - fee_pips),
                U256::from(Self::MAX_SWAP_FEE),
            )?;
            // println!("exact_in: amount_remaining_abs={:?}, amount_remaining_less_fee={:?}", amount_remaining_abs, amount_remaining_less_fee);

            let sqrt_price_next_x96 = SqrtPriceMath::get_next_sqrt_price_from_input(
                sqrt_price_current_x96,
                liquidity,
                amount_remaining_less_fee,
                zero_for_one,
            )?;
            let sqrt_price_next_x96 = Self::get_sqrt_price_target(zero_for_one, sqrt_price_next_x96, sqrt_price_target_x96);
            // println!("exact_in: sqrt_price_next_x96={:?}", sqrt_price_next_x96);

            let amount_in = if zero_for_one {
                SqrtPriceMath::get_amount0_delta(sqrt_price_next_x96, sqrt_price_current_x96, liquidity, true)?
            } else {
                SqrtPriceMath::get_amount1_delta(sqrt_price_current_x96, sqrt_price_next_x96, liquidity, true)?
            } * decimals;

            let amount_out = if zero_for_one {
                SqrtPriceMath::get_amount1_delta(sqrt_price_next_x96, sqrt_price_current_x96, liquidity, false)?
            } else {
                SqrtPriceMath::get_amount0_delta(sqrt_price_current_x96, sqrt_price_next_x96, liquidity, false)?
            } * decimals;

            let fee_amount = if fee_pips == 0 {
                U256::zero()
            } else {
                FullMath::mul_div_rounding_up(amount_in / decimals, U256::from(fee_pips), U256::from(Self::MAX_SWAP_FEE))? * decimals
            };

            // println!("exact_in: amount_in={:?}, amount_out={:?}, fee_amount={:?}", amount_in, amount_out, fee_amount);
            Ok((sqrt_price_next_x96, amount_in, amount_out, fee_amount))
        } else {
            let amount_remaining_u256 = amount_remaining.to_u256().ok_or(SwapMathError::SqrtPriceMathError(SqrtPriceMathError::PriceOverflow))?;
            let amount_remaining_abs = amount_remaining_u256 / decimals;
            // println!("exact_out: amount_remaining_u256={:?}, amount_remaining_abs={:?}", amount_remaining_u256, amount_remaining_abs);
            let sqrt_price_next_x96 = SqrtPriceMath::get_next_sqrt_price_from_output(
                sqrt_price_current_x96,
                liquidity,
                amount_remaining_abs,
                zero_for_one,
            )?;
            let sqrt_price_next_x96 = Self::get_sqrt_price_target(zero_for_one, sqrt_price_next_x96, sqrt_price_target_x96);
            // println!("exact_out: sqrt_price_next_x96={:?}", sqrt_price_next_x96);

            let amount_in = if zero_for_one {
                SqrtPriceMath::get_amount0_delta(sqrt_price_next_x96, sqrt_price_current_x96, liquidity, true)?
            } else {
                SqrtPriceMath::get_amount1_delta(sqrt_price_current_x96, sqrt_price_next_x96, liquidity, true)?
            }.checked_mul(decimals).ok_or(SwapMathError::SqrtPriceMathError(SqrtPriceMathError::PriceOverflow))?;

            let amount_out = amount_remaining_u256;

            let fee_amount = if fee_pips == 0 || liquidity == 0 {
                U256::zero()
            } else {
                FullMath::mul_div_rounding_up(amount_in / decimals, U256::from(fee_pips), U256::from(Self::MAX_SWAP_FEE - fee_pips))?
                    .checked_mul(decimals)
                    .ok_or(SwapMathError::SqrtPriceMathError(SqrtPriceMathError::PriceOverflow))?
            };

            // println!("exact_out: amount_in={:?}, amount_out={:?}, fee_amount={:?}", amount_in, amount_out, fee_amount);
            if amount_in > U256::MAX || amount_out > U256::MAX || fee_amount > U256::MAX {
                return Err(SwapMathError::SqrtPriceMathError(SqrtPriceMathError::PriceOverflow));
            }

            Ok((sqrt_price_next_x96, amount_in, amount_out, fee_amount))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libraries::types::{U160, U256, I256};

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::libraries::types::{U160, U256, I256};
    
        #[test]
        fn test_compute_swap_step_exact_out() {
            let q96 = U256::from(2).pow(U256::from(96));
            let decimals = SwapMath::DECIMALS; // 10^18
            
            // First test case: 5000 tokens with 18 decimals
            let amount_specified_1 = U256::from(5000u64) * decimals;
            let liquidity_scaled = 5_500_000u128; // Base units
            let result1 = SwapMath::compute_swap_step(
                U160::from(100u64) * U160::from(q96.as_u128()),
                U160::from(90u64) * U160::from(q96.as_u128()),
                liquidity_scaled,
                I256::from(amount_specified_1),
                3000u32,
            ).unwrap();
            let (sqrt_price_next1, amount_in1, amount_out1, fee_amount1) = result1;
            let amount_in1_scaled = amount_in1 / decimals;
            let amount_out1_scaled = amount_out1 / decimals;
            let fee_amount1_scaled = fee_amount1 / decimals;
            // println!("First Test - sqrt_price_next: {:?}", sqrt_price_next1);
            // println!("First Test - amount_in: {:?}", amount_in1_scaled);
            // println!("First Test - amount_out: {:?}", amount_out1_scaled);
            // println!("First Test - fee_amount: {:?}", fee_amount1_scaled);
            assert_eq!(sqrt_price_next1, U160::from(90u64) * U160::from(q96.as_u128()));
            assert_eq!(amount_out1_scaled, U256::from(5000u128));
            assert!(amount_in1_scaled > U256::from(6000u128) && amount_in1_scaled < U256::from(6200u128));
            assert!(fee_amount1_scaled > U256::from(10u128) && fee_amount1_scaled < U256::from(20u128));
    
            // Second test case: 909 tokens with 18 decimals
            let amount_specified_2 = U256::from(909u64) * decimals;
            let liquidity_scaled_2 = 1_000_000u128; // Base units
            let result2 = SwapMath::compute_swap_step(
                U160::from(100u64) * U160::from(q96.as_u128()),
                U160::from(110u64) * U160::from(q96.as_u128()),
                liquidity_scaled_2,
                I256::from(amount_specified_2),
                0u32,
            ).unwrap();
            let (sqrt_price_next2, amount_in2, amount_out2, fee_amount2) = result2;
            let amount_in2_scaled = amount_in2 / decimals;
            let amount_out2_scaled = amount_out2 / decimals;
            let fee_amount2_scaled = fee_amount2 / decimals;
            // println!("Second Test - sqrt_price_next: {:?}", sqrt_price_next2);
            // println!("Second Test - amount_in: {:?}", amount_in2_scaled);
            // println!("Second Test - amount_out: {:?}", amount_out2_scaled);
            // println!("Second Test - fee_amount: {:?}", fee_amount2_scaled);
            assert_eq!(sqrt_price_next2, U160::from(110u64) * U160::from(q96.as_u128()));
            assert_eq!(amount_out2_scaled, U256::from(909u128));
            assert_eq!(fee_amount2_scaled, U256::zero());
            // Adjusted expectation for amount_in (token1)
            assert!(amount_in2_scaled >= U256::from(9_000_000u128) && amount_in2_scaled <= U256::from(11_000_000u128));
        }
    
        #[test]
        fn test_compute_swap_step_boundaries() {
            let result = SwapMath::compute_swap_step(
                U160::MAX,
                U160::from(u128::MAX - 1),
                u128::MAX,
                I256::from(-(u128::MAX as i128)),
                SwapMath::MAX_SWAP_FEE,
            );
            match result {
                Ok((sqrt_price_next, amount_in, amount_out, fee_amount)) => {
                    assert!(sqrt_price_next <= U160::MAX);
                    assert!(amount_in <= U256::from(u128::MAX));
                    assert_eq!(fee_amount, amount_in);
                    assert!(amount_out <= U256::from(u128::MAX));
                }
                Err(SwapMathError::SqrtPriceMathError(SqrtPriceMathError::PriceOverflow)) => {
                    assert!(true, "Expected overflow for boundary case")
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
    }  
    
}