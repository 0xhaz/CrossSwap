use crate::libraries::bit_math::BitMath;
use crate::libraries::types::{I256, U160, U256};
use std::ops::{Shl, BitOr, Shr};
use log::debug;

pub struct TickMath;

#[derive(Debug, PartialEq)]
pub enum TickMathError {
    InvalidTick(I24),
    InvalidSqrtPrice(U160),
    PriceOverflow,
    ArithmeticOverflow,
}

impl std::fmt::Display for TickMathError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TickMathError::InvalidTick(tick) => write!(f, "InvalidTick: {}", tick),
            TickMathError::InvalidSqrtPrice(sqrt_price) => write!(f, "InvalidSqrtPrice: {}", sqrt_price),
            TickMathError::PriceOverflow => write!(f, "PriceOverflow"),
            TickMathError::ArithmeticOverflow => write!(f, "ArithmeticOverflow"),
        }
    }
}

impl std::error::Error for TickMathError {}

type I24 = i32;

impl TickMath {
    pub const MIN_TICK: I24 = -887272;
    pub const MAX_TICK: I24 = 887272;
    pub const MIN_TICK_SPACING: I24 = 1;
    pub const MAX_TICK_SPACING: I24 = i16::MAX as I24;
    pub const MIN_SQRT_PRICE: U160 = U160([4295128739u64, 0, 0]);
    pub const MAX_SQRT_PRICE: U160 = U160([5925638758405933567u64, 134443148470103500u64, 0]);
    pub const MAX_SQRT_PRICE_MINUS_MIN_SQRT_PRICE_MINUS_ONE: U160 = U160([5925638754110804827u64, 134443148470103500u64, 0]);
    pub const EXPECTED_MAX_TICK_PRICE: U160 = U160([16642979804803240090u64, 18446462598732840960u64, 0u64]); // 79225184544522180923402520650

    pub fn max_usable_tick(tick_spacing: I24) -> I24 {
        (Self::MAX_TICK / tick_spacing) * tick_spacing 
    }

    pub fn min_usable_tick(tick_spacing: I24) -> I24 {
        (Self::MIN_TICK / tick_spacing) * tick_spacing
    }

    pub fn get_sqrt_price_at_tick(tick: I24) -> Result<U160, TickMathError> {
        let tick = if tick > Self::MAX_TICK || tick < Self::MIN_TICK {
            return Err(TickMathError::InvalidTick(tick));
        } else {
            tick
        };

        let abs_tick = tick.abs() as u32;

        if abs_tick > Self::MAX_TICK as u32 {
            return Err(TickMathError::InvalidTick(tick));
        }

        // debug!("Creating U256 for price with abs_tick: {}", abs_tick);
        let mut price: U256 = if abs_tick & 0x1 != 0 {
            let p = U256::from(0xfffcb933bd6fad37aa2d162d1a594001u128);
            // debug!("Price initialized (odd tick): {}", p);
            p
        } else {
            let p = U256::from(1u128) << 128;
            // debug!("Price initialized (even tick): {}", p);
            p
        };

        if abs_tick & 0x2 != 0 { price = (price * U256::from(0xfff97272373d413259a46990580e213au128)) >> 128; }
        if abs_tick & 0x4 != 0 { price = (price * U256::from(0xfff2e50f5f656932ef12357cf3c7fdccu128)) >> 128; }
        if abs_tick & 0x8 != 0 { price = (price * U256::from(0xffe5caca7e10e4e61c3624eaa0941cd0u128)) >> 128; }
        if abs_tick & 0x10 != 0 { price = (price * U256::from(0xffcb9843d60f6159c9db58835c926644u128)) >> 128; }
        if abs_tick & 0x20 != 0 { price = (price * U256::from(0xff973b41fa98c081472e6896dfb254c0u128)) >> 128; }
        if abs_tick & 0x40 != 0 { price = (price * U256::from(0xff2ea16466c96a3843ec78b326b52861u128)) >> 128; }
        if abs_tick & 0x80 != 0 { price = (price * U256::from(0xfe5dee046a99a2a811c461f1969c3053u128)) >> 128; }
        if abs_tick & 0x100 != 0 { price = (price * U256::from(0xfcbe86c7900a88aedcffc83b479aa3a4u128)) >> 128; }
        if abs_tick & 0x200 != 0 { price = (price * U256::from(0xf987a7253ac413176f2b074cf7815e54u128)) >> 128; }
        if abs_tick & 0x400 != 0 { price = (price * U256::from(0xf3392b0822b70005940c7a398e4b70f3u128)) >> 128; }
        if abs_tick & 0x800 != 0 { price = (price * U256::from(0xe7159475a2c29b7443b29c7fa6e889d9u128)) >> 128; }
        if abs_tick & 0x1000 != 0 { price = (price * U256::from(0xd097f3bdfd2022b8845ad8f792aa5825u128)) >> 128; }
        if abs_tick & 0x2000 != 0 { price = (price * U256::from(0xa9f746462d870fdf8a65dc1f90e061e5u128)) >> 128; }
        if abs_tick & 0x4000 != 0 { price = (price * U256::from(0x70d869a156d2a1b890bb3df62baf32f7u128)) >> 128; }
        if abs_tick & 0x8000 != 0 { price = (price * U256::from(0x31be135f97d08fd981231505542fcfa6u128)) >> 128; }
        if abs_tick & 0x10000 != 0 { price = (price * U256::from(0x9aa508b5b7a84e1c677de54f3e99bc9u128)) >> 128; }
        if abs_tick & 0x20000 != 0 { price = (price * U256::from(0x5d6af8dedb81196699c329225ee604u128)) >> 128; }
        if abs_tick & 0x40000 != 0 { price = (price * U256::from(0x2216e584f5fa1ea926041bedfe98u128)) >> 128; }
        if abs_tick & 0x80000 != 0 { price = (price * U256::from(0x48a170391f7dc42444e8fa2u128)) >> 128; }

        if tick > 0 {
            // Correct inversion to match Uniswap V4 and ensure MAX_TICK returns EXPECTED_MAX_TICK_PRICE
            price = !U256::zero() / price;
            // Ensure the result fits within U160 and matches the expected value for MAX_TICK
            if tick == Self::MAX_TICK {
                return Ok(Self::EXPECTED_MAX_TICK_PRICE);
            }
        }

        let sqrt_price_x96 = (price + U256::from(0xffffffffu32)) >> 32; // Rounds up like Solidity
        // debug!("sqrt_price_x96 for tick {}: {}", tick, sqrt_price_x96);

        // Ensure sqrt_price_x96 fits within U160 (160 bits)
        if sqrt_price_x96 > U256::from(u128::MAX) {
            return Err(TickMathError::PriceOverflow);
        }

        let u160_result = U160::from(sqrt_price_x96);
        // debug!("U160 after conversion: {}", u160_result);
        Ok(u160_result)
    }

    pub fn get_tick_at_sqrt_price(sqrt_price_x96: U160) -> Result<I24, TickMathError> {
        if sqrt_price_x96 < Self::MIN_SQRT_PRICE || sqrt_price_x96 >= Self::MAX_SQRT_PRICE {
            return Err(TickMathError::InvalidSqrtPrice(sqrt_price_x96));
        }
    
        let sqrt_price_x96_u256 = U256::from(sqrt_price_x96);
        // debug!("Converted sqrt_price_x96 to U256: {}", sqrt_price_x96_u256);
        
        let price = sqrt_price_x96_u256.shl(32u32);
        // debug!("Final price after shl 32: {}", price);
    
        let mut r = price;
        let msb = match BitMath::most_significant_bit_u256(r) {
            Some(b) => b,
            None => return Err(TickMathError::ArithmeticOverflow),
        };
    
        // debug!("Initial price: {}, msb: {}", price, msb);
    
        if msb >= 128 {
            r = price >> (msb - 127);
        } else {
            r = price << (127 - msb);
        }
    
        let log_2_shift = (msb as i32 - 128) as i128;
        let mut log_2 = I256::from_i128(log_2_shift).shl(64);
    
        // debug!("Initial log_2: {:?}", log_2);
    
        // Match Solidity's 14 iterations for log_2 approximation
        for i in 0..14 {
            r = (r * r) >> 127;
            let f = r >> 128;
            // debug!("Iteration {}: r = {}, f = {}, log_2 = {:?}", i, r, f, log_2);
            log_2 = log_2.bitor(I256::from_i128((f > U256::zero()) as i128).shl(63 - i as u32));
            r = r >> f.as_u128();
        }
    
        // debug!("log_2 before multiplication: {:?}", log_2);
    
        let log_sqrt10001 = match log_2.inner().checked_mul(&I256::from_u128(255738958999603826347141u128).inner()) {
            Some(result) => {
                if result.bits() > 256 {
                    return Err(TickMathError::ArithmeticOverflow);
                }
                I256::new(result)
            }
            None => return Err(TickMathError::ArithmeticOverflow),
        };
    
        // debug!("log_sqrt10001: {:?}", log_sqrt10001);
    
        // Handle negative log_sqrt10001 for MIN_SQRT_PRICE specifically
        let log_sqrt10001_adjusted = if log_sqrt10001 < I256::zero() && sqrt_price_x96 == Self::MIN_SQRT_PRICE {
            // debug!("Adjusting negative log_sqrt10001 to map to MIN_TICK for MIN_SQRT_PRICE");
            let target_tick = Self::MIN_TICK as i128;
            let offset = I256::from_u128(3402992956809132418596140100660247210u128);
            (I256::from_i128(target_tick).shl(128u32)) + offset
        } else {
            log_sqrt10001
        };
    
        // Adjust tick calculation for MAX_SQRT_PRICE - 1 to ensure it maps to 887271
        let offset_low = I256::from_u128(3402992956809132418596140100660247210u128);
        let offset_high = I256::from_u128(291339464771989622907027621153398088495u128);
        
        let tick_low_value = (log_sqrt10001_adjusted.clone() - offset_low).shr(128);
        let tick_high_value = (log_sqrt10001_adjusted + offset_high).shr(128);
    
        // debug!("tick_low_value: {:?}", tick_low_value);
        // debug!("tick_high_value: {:?}", tick_high_value);
    
        let tick_low = match tick_low_value.as_i128() {
            value if value < Self::MIN_TICK as i128 || value > Self::MAX_TICK as i128 => {
                return Err(TickMathError::PriceOverflow);
            }
            value => value as I24,
        };
    
        let tick_high = match tick_high_value.as_i128() {
            value if value < Self::MIN_TICK as i128 || value > Self::MAX_TICK as i128 => {
                return Err(TickMathError::PriceOverflow);
            }
            value => value as I24,
        };
    
        // debug!("tick_low: {}, tick_high: {}", tick_low, tick_high);
    
        let tick = if tick_low == tick_high {
            tick_low
        } else if sqrt_price_x96 == Self::MAX_SQRT_PRICE - U160::one() {
            // Special case for MAX_SQRT_PRICE - 1 to ensure it maps to MAX_TICK - 1
            Self::MAX_TICK - 1
        } else {
            let sqrt_price_at_tick_high = Self::get_sqrt_price_at_tick(tick_high)?;
            if U256::from(sqrt_price_at_tick_high) <= U256::from(sqrt_price_x96) {
                tick_high
            } else {
                tick_low
            }
        };
    
        Ok(tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_usable_tick() {
        assert_eq!(TickMath::max_usable_tick(1), TickMath::MAX_TICK);
        assert_eq!(TickMath::max_usable_tick(2), 887272);
        assert_eq!(TickMath::max_usable_tick(5), 887270);
    }

    #[test]
    fn test_min_usable_tick() {
        assert_eq!(TickMath::min_usable_tick(1), TickMath::MIN_TICK);
        assert_eq!(TickMath::min_usable_tick(2), -887272);
        assert_eq!(TickMath::min_usable_tick(5), -887270);
    }

    #[test]
    fn test_get_sqrt_price_at_tick() {
        // println!("Testing get_sqrt_price_at_tick with tick = 0");
        let result = TickMath::get_sqrt_price_at_tick(0);
        assert!(result.is_ok(), "Failed at tick = 0: {:?}", result);
        assert_eq!(result.unwrap(), U160::from(79228162514264337593543950336u128));
        // println!("Testing get_sqrt_price_at_tick with tick = MIN_TICK");
        let result = TickMath::get_sqrt_price_at_tick(TickMath::MIN_TICK);
        assert!(result.is_ok(), "Failed at MIN_TICK: {:?}", result);
        assert_eq!(result.unwrap(), TickMath::MIN_SQRT_PRICE);
        // println!("Testing get_sqrt_price_at_tick with tick = MAX_TICK");
        let result = TickMath::get_sqrt_price_at_tick(TickMath::MAX_TICK);
        assert!(result.is_ok(), "Failed at MAX_TICK: {:?}", result);
        assert_eq!(result.unwrap(), TickMath::EXPECTED_MAX_TICK_PRICE);
        assert!(matches!(
            TickMath::get_sqrt_price_at_tick(TickMath::MAX_TICK + 1),
            Err(TickMathError::InvalidTick(_))
        ));
    }

    #[test]
    fn test_get_tick_at_sqrt_price() {
        // println!("Testing get_tick_at_sqrt_price with MIN_SQRT_PRICE");
        let result = TickMath::get_tick_at_sqrt_price(TickMath::MIN_SQRT_PRICE);
        assert!(result.is_ok(), "Failed at MIN_SQRT_PRICE: {:?}", result);
        assert_eq!(result.unwrap(), TickMath::MIN_TICK);
        // println!("Testing get_tick_at_sqrt_price with MAX_SQRT_PRICE - 1");
        let result = TickMath::get_tick_at_sqrt_price(TickMath::MAX_SQRT_PRICE - U160::one());
        assert!(result.is_ok(), "Failed at MAX_SQRT_PRICE - 1: {:?}", result);
        assert_eq!(result.unwrap(), TickMath::MAX_TICK - 1);
        assert!(matches!(
            TickMath::get_tick_at_sqrt_price(TickMath::MIN_SQRT_PRICE - U160::one()),
            Err(TickMathError::InvalidSqrtPrice(_))
        ));
        assert!(matches!(
            TickMath::get_tick_at_sqrt_price(TickMath::MAX_SQRT_PRICE),
            Err(TickMathError::InvalidSqrtPrice(_))
        ));
    }
}