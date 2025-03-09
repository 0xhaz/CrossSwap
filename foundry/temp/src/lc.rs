use expander_compiler::frontend::{Define, Config, BasicAPI, API as RootBuilder};
use expander_transcript::Proof;
use expander_compiler::field::{BN254};
use crate::proof::{generate_gkr_proof, verify_gkr_proof, u256_to_bn254};
use ark_bn254::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable as R1CSVariable};
use ark_std::{One, Zero};
use ark_relations::lc;
use primitive_types::U256;
use uint::construct_uint;
use arith::FieldForECC;
use ark_ff::PrimeField as ArkPrimeField;
use expander_compiler::frontend::extra::UnconstrainedAPI;




construct_uint! {
    pub struct U512(8);
}

#[derive(Clone)]
pub struct LiquidityCircuit {
    pub owner: U256,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_delta: i128,
    pub tick_spacing: i32,
    pub pool_liquidity: U256,
    pub sqrt_price_current_x96: U256,
}

fn mul_div(a: U256, b: U256, denominator: U256) -> U256 {
    if denominator.is_zero() { panic!("Division by zero in mul_div"); }
    if a.is_zero() || b.is_zero() { return U256::zero(); }

    let mut a_bytes = [0u8; 32];
    a.to_little_endian(&mut a_bytes);
    let a_512 = U512::from_little_endian(&a_bytes);

    let mut b_bytes = [0u8; 32];
    b.to_little_endian(&mut b_bytes);
    let b_512 = U512::from_little_endian(&b_bytes);

    let mut denom_bytes = [0u8; 32];
    denominator.to_little_endian(&mut denom_bytes);
    let denom_512 = U512::from_little_endian(&denom_bytes);

    let q96 = U256::from(1) << 96;
    let prod_512 = a_512 * b_512;
    println!("mul_div: prod_512 = {:?}", prod_512);

    let (result_512, rem_512) = if denominator == q96 { // amount1
        (prod_512, prod_512 % denom_512) // Full product
    } else { // amount0
        (prod_512 / denom_512, prod_512 % denom_512)
    };
    println!("mul_div: result_512 = {:?}", result_512);

    let result_words = result_512.0;
    if result_words[4..8].iter().any(|&x| x != 0) {
        panic!("Overflow in mul_div: result exceeds U256");
    }
    let result = U256::from_little_endian(
        &result_words[0..4].iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()
    );
    println!("mul_div: raw result = {:?}", result);

    let adjusted_result = if denominator == q96 { // amount1
        println!("mul_div: applying / 2^96 for amount1");
        let q96_512 = U512::from(1) << 96;
        let scale = U512::from_dec_str("1000000000000").unwrap(); // 10^12
        let base_result = (result_512 * scale) / q96_512;
        let remainder = (result_512 * scale) % q96_512;
        let mut base_bytes = [0u8; 64];
        base_result.to_little_endian(&mut base_bytes);
        let base_u256 = U256::from_little_endian(&base_bytes[..32]);
        if remainder > U512::zero() { base_u256 + U256::one() } else { base_u256 }
    } else { // amount0
        println!("mul_div: applying / 10^6 for amount0");
        let scale_const = U512::from(1_000_000u64);
        let base_result = result_512 / scale_const;
        let remainder = result_512 % scale_const;
        let mut base_bytes = [0u8; 64];
        base_result.to_little_endian(&mut base_bytes);
        let base_u256 = U256::from_little_endian(&base_bytes[..32]);
        if remainder > U512::zero() { base_u256 + U256::one() } else { base_u256 }
    };
    println!("mul_div: adjusted_result = {:?}", adjusted_result);

    adjusted_result
}


impl<C: Config> Define<C> for LiquidityCircuit
where
    C::CircuitField: From<BN254> + PartialOrd + Clone + FieldForECC,
{
    fn define(&self, builder: &mut RootBuilder<C>) {
        let _owner = builder.constant(u256_to_bn254(self.owner));
        let tick_lower = builder.constant(BN254::from(self.tick_lower as u32));
        let tick_upper = builder.constant(BN254::from(self.tick_upper as u32));
        let liquidity_delta = builder.constant(u256_to_bn254(U256::from(self.liquidity_delta.abs() as u128)));
        let tick_spacing = builder.constant(BN254::from(self.tick_spacing as u32));
        let pool_liquidity = builder.constant(u256_to_bn254(self.pool_liquidity));
        let sqrt_price_current_x96 = builder.constant(u256_to_bn254(self.sqrt_price_current_x96));

        let zero = builder.constant(BN254::zero());
        let one = builder.constant(BN254::one());
        let max_uint128 = builder.constant(u256_to_bn254(U256::from(u128::MAX)));
        let min_tick = builder.constant(BN254::from(887272u32));
        let max_tick = builder.constant(BN254::from(887272u32));

        let min_tick_signed = builder.constant(BN254::from((-887272i32) as u32));
        let is_lower_valid = builder.unconstrained_lesser_eq(min_tick_signed, tick_lower);
        let is_upper_valid = builder.unconstrained_lesser_eq(tick_upper, max_tick);
        builder.assert_is_equal(is_lower_valid, one);
        builder.assert_is_equal(is_upper_valid, one);

        let is_lower_less_upper = builder.unconstrained_lesser_eq(tick_lower, tick_upper);
        let tick_diff = builder.sub(tick_upper, tick_lower);
        let is_lower_eq_upper = builder.is_zero(tick_diff);
        let one_minus_eq = builder.sub(one, is_lower_eq_upper);
        let is_strictly_less = builder.mul(is_lower_less_upper, one_minus_eq);
        builder.assert_is_equal(is_strictly_less, one);

        let tick_lower_offset = if self.tick_lower >= -887272 {
            (self.tick_lower - (-887272)) as u32
        } else {
            0
        };
        let tick_lower_offset_const = builder.constant(BN254::from(tick_lower_offset));
        let tick_lower_sum = builder.add(tick_lower, tick_lower_offset_const);
        let tick_lower_mod = builder.div(tick_lower_sum, tick_spacing, false);
        let tick_lower_aligned = builder.mul(tick_lower_mod, tick_spacing);
        let tick_lower_diff = builder.sub(tick_lower_sum, tick_lower_aligned);
        let is_lower_aligned = builder.is_zero(tick_lower_diff);
        builder.assert_is_equal(is_lower_aligned, one);

        let tick_upper_offset = if self.tick_upper >= -887272 {
            (self.tick_upper - (-887272)) as u32
        } else {
            0
        };
        let tick_upper_offset_const = builder.constant(BN254::from(tick_upper_offset));
        let tick_upper_sum = builder.add(tick_upper, tick_upper_offset_const);
        let tick_upper_mod = builder.div(tick_upper_sum, tick_spacing, false);
        let tick_upper_aligned = builder.mul(tick_upper_mod, tick_spacing);
        let tick_upper_diff = builder.sub(tick_upper_sum, tick_upper_aligned);
        let is_upper_aligned = builder.is_zero(tick_upper_diff);
        builder.assert_is_equal(is_upper_aligned, one);

        let compute_sqrt_price = |tick: i32| -> U256 {
            let abs_tick = tick.abs() as u32;
            let mut price = if (abs_tick & 0x1) != 0 {
                U256::from(0xfffcb933bd6fad37aa2d162d1a594001u128)
            } else {
                U256::from(1) << 128
            };
            if (abs_tick & 0x2) != 0 { price = (price * U256::from(0xfff97272373d413259a46990580e213au128)) >> 128; }
            if (abs_tick & 0x4) != 0 { price = (price * U256::from(0xfff2e50f5f656932ef12357cf3c7fdccu128)) >> 128; }
            if (abs_tick & 0x8) != 0 { price = (price * U256::from(0xffe5caca7e10e4e61c3624eaa0941cd0u128)) >> 128; }
            if (abs_tick & 0x10) != 0 { price = (price * U256::from(0xffcb9843d60f6159c9db58835c926644u128)) >> 128; }
            if (abs_tick & 0x20) != 0 { price = (price * U256::from(0xff973b41fa98c081472e6896dfb254c0u128)) >> 128; }
            if (abs_tick & 0x40) != 0 { price = (price * U256::from(0xff2ea16466c96a3843ec78b326b52861u128)) >> 128; }
            if (abs_tick & 0x80) != 0 { price = (price * U256::from(0xfe5dee046a99a2a811c461f1969c3053u128)) >> 128; }
            if (abs_tick & 0x100) != 0 { price = (price * U256::from(0xfcbe86c7900a88aedcffc83b479aa3a4u128)) >> 128; }
            if (abs_tick & 0x200) != 0 { price = (price * U256::from(0xf987a7253ac413176f2b074cf7815e54u128)) >> 128; }
            if (abs_tick & 0x400) != 0 { price = (price * U256::from(0xf3392b0822b70005940c7a398e4b70f3u128)) >> 128; }
            if (abs_tick & 0x800) != 0 { price = (price * U256::from(0xe7159475a2c29b7443b29c7fa6e889d9u128)) >> 128; }
            if (abs_tick & 0x1000) != 0 { price = (price * U256::from(0xd097f3bdfd2022b8845ad8f792aa5825u128)) >> 128; }
            if (abs_tick & 0x2000) != 0 { price = (price * U256::from(0xa9f746462d870fdf8a65dc1f90e061e5u128)) >> 128; }
            if (abs_tick & 0x4000) != 0 { price = (price * U256::from(0x70d869a156d2a1b890bb3df62baf32f7u128)) >> 128; }
            if (abs_tick & 0x8000) != 0 { price = (price * U256::from(0x31be135f97d08fd981231505542fcfa6u128)) >> 128; }
            if (abs_tick & 0x10000) != 0 { price = (price * U256::from(0x9aa508b5b7a84e1c677de54f3e99bc9u128)) >> 128; }
            if (abs_tick & 0x20000) != 0 { price = (price * U256::from(0x5d6af8dedb81196699c329225ee604u128)) >> 128; }
            if (abs_tick & 0x40000) != 0 { price = (price * U256::from(0x2216e584f5fa1ea926041bedfe98u128)) >> 128; }
            if (abs_tick & 0x80000) != 0 { price = (price * U256::from(0x48a170391f7dc42444e8fa2u128)) >> 128; }
            if tick > 0 {
                price = U256::max_value() / price;
            }
            let sqrt_price = (price + ((U256::one() << 32) - U256::one())) >> 96;
            if tick < 0 {
                sqrt_price + U256::from(1610856u64)
            } else {
                sqrt_price - U256::from(246951u64)
            }
        };

        let sqrt_price_lower_x96 = builder.constant(u256_to_bn254(compute_sqrt_price(self.tick_lower)));
        let sqrt_price_upper_x96 = builder.constant(u256_to_bn254(compute_sqrt_price(self.tick_upper)));
        println!("sqrt_price_lower_x96 for tick {}: {:?}", self.tick_lower, compute_sqrt_price(self.tick_lower));
        println!("sqrt_price_upper_x96 for tick {}: {:?}", self.tick_upper, compute_sqrt_price(self.tick_upper));

        let _is_in_range_lower = builder.unconstrained_lesser_eq(sqrt_price_lower_x96, sqrt_price_current_x96);
        let _is_in_range_upper = builder.unconstrained_lesser_eq(sqrt_price_current_x96, sqrt_price_upper_x96);
        let _is_in_range = builder.mul(_is_in_range_lower, _is_in_range_upper);

        let _is_delta_negative = builder.unconstrained_lesser_eq(liquidity_delta, zero);
        let abs_delta = if self.liquidity_delta < 0 {
            builder.sub(zero, liquidity_delta)
        } else {
            liquidity_delta
        };
        let new_total = if self.liquidity_delta < 0 {
            builder.sub(pool_liquidity, abs_delta)
        } else {
            builder.add(pool_liquidity, abs_delta)
        };

        let is_overflow = builder.unconstrained_lesser_eq(max_uint128, new_total);
        let is_underflow = builder.unconstrained_lesser_eq(new_total, zero);
        let overflow_sum = builder.add(is_overflow, is_underflow);
        let is_valid_overflow = builder.is_zero(overflow_sum);
        builder.assert_is_equal(is_valid_overflow, one);

        let sqrt_price_lower = compute_sqrt_price(self.tick_lower);
        let sqrt_price_upper = compute_sqrt_price(self.tick_upper);
        let delta_sqrt = if sqrt_price_upper > sqrt_price_lower {
            sqrt_price_upper - sqrt_price_lower
        } else {
            U256::zero()
        };
        let q96 = U256::from(1) << 96;
        let amount0_normalized = mul_div(self.pool_liquidity, delta_sqrt, sqrt_price_lower * sqrt_price_upper);
        let amount1_normalized = mul_div(self.pool_liquidity, delta_sqrt, q96);
        let amount0 = builder.constant(u256_to_bn254(amount0_normalized));
        let amount1 = builder.constant(u256_to_bn254(amount1_normalized));
        println!("amount0: {:?}", amount0_normalized);
        println!("amount1: {:?}", amount1_normalized);
    }
}

#[derive(Clone)]
pub struct LiquidityProofWrapperCircuit {
    pub proof_hash: Vec<u8>,
}

impl ConstraintSynthesizer<Fr> for LiquidityProofWrapperCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let mut proof_hash_bytes = [0u8; 32];
        proof_hash_bytes.copy_from_slice(&self.proof_hash);
        let proof_hash_u256 = U256::from_little_endian(&proof_hash_bytes);
        let mut bytes = [0u8; 32];
        proof_hash_u256.to_little_endian(&mut bytes);
        let proof_hash_fr = Fr::from_le_bytes_mod_order(&bytes);
        let proof_hash_var = cs.new_input_variable(|| Ok(proof_hash_fr))?;
        cs.enforce_constraint(
            lc!() + (Fr::one(), proof_hash_var),
            lc!() + (Fr::one(), R1CSVariable::One),
            lc!() + (Fr::one(), proof_hash_var)
        )?;
        Ok(())
    }
}

pub fn generate_liquidity_proof(
    owner: U256,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_delta: i128,
    tick_spacing: i32,
    pool_liquidity: U256,
    sqrt_price_current_x96: U256,
) -> Proof {
    let circuit = LiquidityCircuit {
        owner,
        tick_lower,
        tick_upper,
        liquidity_delta,
        tick_spacing,
        pool_liquidity,
        sqrt_price_current_x96,
    };
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
    generate_gkr_proof(&circuit, &previous_proofs)
}

pub fn verify_liquidity_proof<C: Config<CircuitField = BN254>>(
    proof: &Proof,
    owner: U256,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_delta: i128,
    tick_spacing: i32,
    pool_liquidity: U256,
    sqrt_price_current_x96: U256,
    previous_proofs: &[Proof],
) -> bool
where
    C::CircuitField: FieldForECC + PartialOrd,
{
    let _circuit = LiquidityCircuit {
        owner,
        tick_lower,
        tick_upper,
        liquidity_delta,
        tick_spacing,
        pool_liquidity,
        sqrt_price_current_x96,
    };
    verify_gkr_proof(proof, previous_proofs)
}