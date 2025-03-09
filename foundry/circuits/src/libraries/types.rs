use uint::construct_uint;
use num_bigint::{BigInt, Sign};
use num_traits::{Zero, One, ToPrimitive, FromPrimitive, Signed, Num};
use std::ops::{Shl, Shr, Neg, Add, Sub, BitOr, Mul, Div, Rem};
use std::fmt;
use std::str::FromStr;
use log::debug;

construct_uint! {
    pub struct U256(4); // 4 * 64 = 256 bits
}

construct_uint! {
    pub struct U160(3); // 3 * 64 = 192 bits (160 bits padded to 24 bytes)
}

impl From<U160> for U256 {
    fn from(value: U160) -> Self {
        let mut value_bytes = [0u8; 24];
        value.to_little_endian(&mut value_bytes);
        // debug!("U160 to U256: value_bytes = {:?}", &value_bytes[..]);
        
        let mut u256_bytes = [0u8; 32];
        u256_bytes[..24].copy_from_slice(&value_bytes); // Pad with zeros
        let u256_value = U256::from_little_endian(&u256_bytes);
        // debug!("U256 from U160: {}", u256_value);
        u256_value
    }
}

impl From<U256> for U160 {
    fn from(value: U256) -> Self {
        let mut bytes = [0u8; 32]; // 32 bytes from U256
        value.to_little_endian(&mut bytes);
        let mut u160_bytes = [0u8; 24]; // 24 bytes for U160 (160 bits padded to 3 * 64-bit words)
        u160_bytes[..20].copy_from_slice(&bytes[..20]); // Copy first 20 bytes (160 bits), pad with zeros
        // debug!("U256 to U160: bytes = {:?}", &bytes[..]);
        // debug!("U256 to U160: u160_bytes = {:?}", &u160_bytes[..]);
        let u160 = U160::from_little_endian(&u160_bytes);
        // debug!("U160 from U256: {}", u160);
        u160
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct I256(BigInt);

impl I256 {
    pub fn to_u256(&self) -> Option<U256> {
        if *self < I256::zero() {
            // println!("I256::to_u256: negative value, returning None");
            None
        } else {
            let (sign, bytes) = self.0.to_bytes_le();
            if sign == Sign::Minus {
                // println!("I256::to_u256: sign is Minus, returning None");
                None
            } else {
                let mut padded = [0u8; 32];
                let len = bytes.len().min(32);
                padded[..len].copy_from_slice(&bytes[..len]);
                // println!("I256::to_u256: bytes={:?}, padded={:?}", bytes, padded);
                let result = U256::from_little_endian(&padded);
                // println!("I256::to_u256: result={:?}", result);
                Some(result)
            }
        }
    }

    pub fn zero() -> Self {
        I256(BigInt::zero())
    }

    pub fn one() -> Self {
        I256(BigInt::one())
    }

    /// Creates an `I256` from an `i128` value.
    pub fn from_i128(value: i128) -> Self {
        I256(BigInt::from_i128(value).unwrap_or_else(|| {
            if value < 0 {
                BigInt::from_i128(value).unwrap_or(BigInt::zero())
            } else {
                BigInt::from_i128(value).unwrap_or(BigInt::zero())
            }
        }))
    }

    /// Creates an `I256` from a `u128` value (unsigned, treated as non-negative).
    pub fn from_u128(value: u128) -> Self {
        I256(BigInt::from_u128(value).unwrap_or(BigInt::zero()))
    }

    /// Retrieves the value as an `i128`, or `i128::MAX` if it exceeds `i128` range.
    pub fn as_i128(&self) -> i128 {
        self.0.to_i128().unwrap_or(i128::MAX)
    }

    /// Access the inner BigInt for advanced operations (e.g., checked arithmetic).
    pub fn inner(&self) -> &BigInt {
        &self.0
    }

    /// Create a new I256 from a BigInt, ensuring it fits within 256 bits.
    pub fn new(value: BigInt) -> Self {
        if value.bits() > 256 {
            panic!("Value exceeds 256 bits");
        }
        I256(value)
    }

    /// Returns the absolute value as a U256.
    pub fn abs(&self) -> U256 {
        let (_sign, bytes) = self.0.abs().to_bytes_le();
        let mut padded = [0u8; 32];
        padded[..bytes.len()].copy_from_slice(&bytes);
        // debug!("I256 abs to U256: padded = {:?}", &padded[..]);
        U256::from_little_endian(&padded)
    }
}

impl From<i128> for I256 {
    fn from(value: i128) -> Self {
        I256::from_i128(value)
    }
}

impl From<u128> for I256 {
    fn from(value: u128) -> Self {
        I256::from_u128(value)
    }
}

impl From<U256> for I256 {
    fn from(value: U256) -> Self {
        let mut bytes = [0u8; 32];
        value.to_little_endian(&mut bytes);
        // debug!("U256 to I256: bytes = {:?}", &bytes[..]);
        I256(BigInt::from_bytes_le(Sign::Plus, &bytes))
    }
}

impl From<I256> for U256 {
    fn from(value: I256) -> Self {
        let (sign, bytes) = value.0.to_bytes_le();
        if sign == Sign::Minus {
            panic!("Cannot convert negative I256 to U256");
        }
        let mut padded = [0u8; 32];
        padded[..bytes.len()].copy_from_slice(&bytes);
        // debug!("I256 to U256: padded = {:?}", &padded[..]);
        U256::from_little_endian(&padded)
    }
}

impl Neg for I256 {
    type Output = Self;

    fn neg(self) -> Self {
        I256(-self.0)
    }
}

impl Add for I256 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        I256(self.0 + other.0)
    }
}

impl Sub for I256 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        I256(self.0 - other.0)
    }
}

impl BitOr for I256 {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        I256(self.0.bitor(&other.0))
    }
}

impl Shl<u32> for I256 {
    type Output = Self;

    fn shl(self, rhs: u32) -> Self {
        I256(self.0 << rhs)
    }
}

impl Shr<u32> for I256 {
    type Output = Self;

    fn shr(self, rhs: u32) -> Self {
        I256(self.0 >> rhs)
    }
}

impl Mul for I256 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        I256(self.0 * other.0)
    }
}

impl Div for I256 {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        if other.is_zero() {
            panic!("Division by zero");
        }
        I256(self.0 / other.0)
    }
}

impl Rem for I256 {
    type Output = Self;

    fn rem(self, other: Self) -> Self {
        if other.is_zero() {
            panic!("Remainder by zero");
        }
        I256(self.0 % other.0)
    }
}

impl fmt::Debug for I256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for I256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for I256 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for I256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl Num for I256 {
    type FromStrRadixErr = num_bigint::ParseBigIntError;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        BigInt::from_str_radix(str, radix).map(|big_int| I256(big_int))
    }
}

impl Signed for I256 {
    fn abs(&self) -> Self {
        I256(self.0.abs())
    }

    fn abs_sub(&self, other: &Self) -> Self {
        if *self <= *other {
            Self::zero()
        } else {
            I256(&self.0 - &other.0)
        }
    }

    fn signum(&self) -> Self {
        I256(self.0.signum())
    }

    fn is_positive(&self) -> bool {
        self.0 > BigInt::zero()
    }

    fn is_negative(&self) -> bool {
        self.0 < BigInt::zero()
    }
}

impl Zero for I256 {
    fn zero() -> Self {
        I256(BigInt::zero())
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl One for I256 {
    fn one() -> Self {
        I256(BigInt::one())
    }
}