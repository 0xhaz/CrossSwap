use expander_compiler::frontend::{Define, Config, BasicAPI, API as RootBuilder};
use expander_transcript::Proof;
use expander_compiler::field::BN254;
use crate::proof::{generate_gkr_proof, verify_gkr_proof, u256_to_bn254};
use ark_bn254::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable as R1CSVariable};
use ark_std::One;
use ark_relations::lc;
use arith::FieldForECC;
use ark_ff::PrimeField as ArkPrimeField;
use expander_compiler::frontend::extra::UnconstrainedAPI;
use crate::libraries::types::{U256, U160, I256};
use crate::libraries::tick_math::{TickMath, TickMathError};
use crate::libraries::sqrt_price_math::SqrtPriceMath;
use crate::libraries::liquidity_math::LiquidityMath;
use crate::libraries::safecast::SafeCast;

#[derive(Clone, Debug)]
pub struct BalanceDelta {
    pub amount0: I256,
    pub amount1: I256,
}

#[derive(Clone)]
pub struct LiquidityCircuit {
    pub owner: U256,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_delta: i128,
    pub tick_spacing: i32,
    pub salt: [u8; 32],
    pub sqrt_price_current_x96: U256,
    pub hook_data: Vec<u8>,
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
        let salt = builder.constant(u256_to_bn254(U256::from_little_endian(&self.salt)));
        let sqrt_price_current_x96 = builder.constant(u256_to_bn254(self.sqrt_price_current_x96));
        let decimals = U256::from(10).pow(U256::from(18)); // Added for scaling

        let zero = builder.constant(BN254::zero());
        let one = builder.constant(BN254::one());
        let min_tick = builder.constant(BN254::from(887272u32));
        let max_tick = builder.constant(BN254::from(887272u32));
        let min_tick_signed = builder.constant(BN254::from((-887272i32) as u32));

        // Validate tick ranges
        let is_lower_valid = builder.unconstrained_lesser_eq(min_tick_signed, tick_lower);
        let is_upper_valid = builder.unconstrained_lesser_eq(tick_upper, max_tick);
        builder.assert_is_equal(is_lower_valid, one);
        builder.assert_is_equal(is_upper_valid, one);

        // Validate tick_lower < tick_upper
        let is_lower_less_upper = builder.unconstrained_lesser_eq(tick_lower, tick_upper);
        let tick_diff = builder.sub(tick_upper, tick_lower);
        let is_lower_eq_upper = builder.is_zero(tick_diff);
        let one_minus_eq = builder.sub(one, is_lower_eq_upper);
        let is_strictly_less = builder.mul(is_lower_less_upper, one_minus_eq);
        builder.assert_is_equal(is_strictly_less, one);

        // Validate tick spacing alignment
        let tick_lower_mod = builder.div(tick_lower, tick_spacing, false);
        let tick_lower_aligned = builder.mul(tick_lower_mod, tick_spacing);
        let tick_lower_diff = builder.sub(tick_lower, tick_lower_aligned);
        let is_lower_aligned = builder.is_zero(tick_lower_diff);
        builder.assert_is_equal(is_lower_aligned, one);

        let tick_upper_mod = builder.div(tick_upper, tick_spacing, false);
        let tick_upper_aligned = builder.mul(tick_upper_mod, tick_spacing);
        let tick_upper_diff = builder.sub(tick_upper, tick_upper_aligned);
        let is_upper_aligned = builder.is_zero(tick_upper_diff);
        builder.assert_is_equal(is_upper_aligned, one);

        // Compute sqrt prices
        let sqrt_price_lower_x96 = TickMath::get_sqrt_price_at_tick(self.tick_lower)
            .unwrap_or_else(|e| {
                println!("TickMath error for lower tick: {:?}", e);
                U160::zero()
            });
        let sqrt_price_upper_x96 = TickMath::get_sqrt_price_at_tick(self.tick_upper)
            .unwrap_or_else(|e| {
                println!("TickMath error for upper tick: {:?}", e);
                U160::zero()
            });

        // Before hook simulation
        let adjusted_liquidity_delta = if !self.hook_data.is_empty() && self.hook_data == vec![1] {
            self.liquidity_delta + (self.liquidity_delta.abs() / 100)
        } else {
            self.liquidity_delta
        };

        // Compute amounts (unscaled)
        let amount0_unscaled = SqrtPriceMath::get_amount0_delta_signed(
            sqrt_price_lower_x96,
            sqrt_price_upper_x96,
            adjusted_liquidity_delta,
        ).unwrap_or_else(|e| {
            println!("SqrtPriceMath error for amount0: {:?}", e);
            I256::zero()
        });
        let amount1_unscaled = SqrtPriceMath::get_amount1_delta_signed(
            sqrt_price_lower_x96,
            sqrt_price_upper_x96,
            adjusted_liquidity_delta,
        ).unwrap_or_else(|e| {
            println!("SqrtPriceMath error for amount1: {:?}", e);
            I256::zero()
        });

        // Scale to token decimals (10^18)
        let amount0 = amount0_unscaled * I256::from(decimals);
        let amount1 = amount1_unscaled * I256::from(decimals);

        // Principal delta
        let principal_delta = BalanceDelta {
            amount0,
            amount1,
        };

        // Fees accrued (placeholder)
        let fees_accrued = BalanceDelta {
            amount0: I256::zero(),
            amount1: I256::zero(),
        };

        // After hook simulation
        let caller_delta = if self.hook_data == vec![2] {
            BalanceDelta {
                amount0: principal_delta.amount0 * I256::from(99i128) / I256::from(100i128),
                amount1: principal_delta.amount1 * I256::from(99i128) / I256::from(100i128),
            }
        } else {
            principal_delta
        };

        // Circuit constraints
        let sqrt_price_lower_x96_field = builder.constant(u256_to_bn254(U256::from(sqrt_price_lower_x96)));
        let sqrt_price_upper_x96_field = builder.constant(u256_to_bn254(U256::from(sqrt_price_upper_x96)));
        let sqrt_price_current_x96_field = builder.constant(u256_to_bn254(self.sqrt_price_current_x96));
        let is_in_range_lower = builder.unconstrained_lesser_eq(sqrt_price_lower_x96_field, sqrt_price_current_x96_field);
        let is_in_range_upper = builder.unconstrained_lesser_eq(sqrt_price_current_x96_field, sqrt_price_upper_x96_field);
        let is_in_range = builder.mul(is_in_range_lower, is_in_range_upper);

        let amount0_field = builder.constant(u256_to_bn254(caller_delta.amount0.abs()));
        let amount1_field = builder.constant(u256_to_bn254(caller_delta.amount1.abs()));

        // Output
        println!("Computed:");
        println!("  sqrt_price_lower_x96: {:?}", sqrt_price_lower_x96);
        println!("  sqrt_price_upper_x96: {:?}", sqrt_price_upper_x96);
        println!("  caller_delta: amount0={:?}, amount1={:?}", caller_delta.amount0, caller_delta.amount1);
        println!("  fees_accrued: amount0={:?}, amount1={:?}", fees_accrued.amount0, fees_accrued.amount1);
        println!("=============================================");
    }
}

pub fn generate_liquidity_proof(
    owner: U256,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_delta: i128,
    tick_spacing: i32,
    salt: [u8; 32],
    sqrt_price_current_x96: U256,
    hook_data: Vec<u8>,
) -> Proof {
    let circuit = LiquidityCircuit {
        owner,
        tick_lower,
        tick_upper,
        liquidity_delta,
        tick_spacing,
        salt,
        sqrt_price_current_x96,
        hook_data,
    };
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
    generate_gkr_proof(&circuit, &previous_proofs)
}

pub fn verify_liquidity_proof(
    proof: &Proof,
    previous_proofs: &[Proof],
) -> bool {
    verify_gkr_proof(proof, previous_proofs)
}