use expander_compiler::frontend::{Define, Config, BasicAPI, API as RootBuilder};
use expander_transcript::Proof;
use expander_compiler::field::BN254;
use crate::proof::{generate_gkr_proof, verify_gkr_proof, u256_to_bn254, CircuitPublicOutputs, GKRProver};
use ark_bn254::Fr;
use expander_compiler::frontend::extra::UnconstrainedAPI;
use crate::libraries::types::{U256, U160, I256};
use crate::libraries::SwapMath;
use arith::FieldForECC;
use num_bigint::BigInt;
use std::any::Any;

#[derive(Clone, Debug)]
pub struct BalanceDelta {
    pub amount0: I256,
    pub amount1: I256,
}

#[derive(Clone)]
pub struct SwapCircuitGKR {
    pub zero_for_one: bool,
    pub amount_specified: I256,
    pub sqrt_price_limit_x96: U160,
    pub sqrt_price_current_x96: U160,
    pub liquidity: u128,
    pub fee_pips: u32,
    pub hook_data: Vec<u8>,
}

impl<C: Config> Define<C> for SwapCircuitGKR
where
    C::CircuitField: From<BN254> + PartialOrd + Clone + FieldForECC,
{
    fn define(&self, builder: &mut RootBuilder<C>) {
        let _zero_for_one = builder.constant(BN254::from(self.zero_for_one as u64));
        let decimals = U256::from(10).pow(U256::from(18));
        let amount_specified = builder.constant(u256_to_bn254(self.amount_specified.abs()));
        let sqrt_price_limit_x96 = builder.constant(u256_to_bn254(U256::from(self.sqrt_price_limit_x96)));
        let sqrt_price_current_x96 = builder.constant(u256_to_bn254(U256::from(self.sqrt_price_current_x96)));
        let liquidity = builder.constant(u256_to_bn254(U256::from(self.liquidity)));
        let fee_pips = builder.constant(BN254::from(self.fee_pips));

        let one = builder.constant(BN254::one());

        let (adjusted_amount, _hook_fee) = if !self.hook_data.is_empty() {
            if self.hook_data == vec![1] {
                let extra_fee = self.amount_specified.abs() / U256::from(100);
                (self.amount_specified.clone(), Some(extra_fee))
            } else {
                (self.amount_specified.clone(), None)
            }
        } else {
            (self.amount_specified.clone(), None)
        };

        let (sqrt_price_next_x96, amount_in, amount_out, fee_amount) = SwapMath::compute_swap_step(
            self.sqrt_price_current_x96,
            self.sqrt_price_limit_x96,
            self.liquidity,
            adjusted_amount,
            self.fee_pips,
        ).unwrap_or_else(|e| {
            println!("SwapMath error in define: {:?}", e);
            (self.sqrt_price_current_x96, U256::zero(), self.amount_specified.abs(), U256::zero())
        });

        let delta = if self.zero_for_one {
            BalanceDelta {
                amount0: I256::from(amount_in) + I256::from(fee_amount),
                amount1: -I256::from(amount_out),
            }
        } else {
            BalanceDelta {
                amount0: -I256::from(amount_out),
                amount1: I256::from(amount_in) + I256::from(fee_amount),
            }
        };

        let final_delta = if self.hook_data == vec![2] {
            if self.zero_for_one {
                BalanceDelta {
                    amount0: delta.amount0,
                    amount1: delta.amount1 * I256::from(99i128) / I256::from(100i128),
                }
            } else {
                BalanceDelta {
                    amount0: delta.amount0 * I256::from(99i128) / I256::from(100i128),
                    amount1: delta.amount1,
                }
            }
        } else {
            delta
        };

        let sqrt_price_next_x96_field = builder.constant(u256_to_bn254(U256::from(sqrt_price_next_x96)));
        let is_valid = if self.zero_for_one {
            builder.unconstrained_lesser_eq(sqrt_price_limit_x96, sqrt_price_next_x96_field)
        } else {
            builder.unconstrained_lesser_eq(sqrt_price_next_x96_field, sqrt_price_limit_x96)
        };
        builder.assert_is_equal(is_valid, one);

        let amount_in_scaled = amount_in / decimals;
        let amount_out_scaled = amount_out;
        let fee_amount_scaled = fee_amount / decimals;
        // println!("Computed:");
        // println!("  sqrt_price_next_x96: {:?}", sqrt_price_next_x96);
        // println!("  amount_in: {:?}", amount_in_scaled);
        // println!("  amount_out: {:?}", amount_out_scaled);
        // println!("  fee_amount: {:?}", fee_amount_scaled);
        // println!("  balance_delta: amount0={:?}, amount1={:?}", final_delta.amount0, final_delta.amount1);
    }
}

impl CircuitPublicOutputs for SwapCircuitGKR {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_public_outputs(&self) -> Vec<U256> {
        let decimals = U256::from(10).pow(U256::from(18));
        let (adjusted_amount, _hook_fee) = if !self.hook_data.is_empty() {
            if self.hook_data == vec![1] {
                let extra_fee = self.amount_specified.abs() / U256::from(100);
                (self.amount_specified.clone(), Some(extra_fee))
            } else {
                (self.amount_specified.clone(), None)
            }
        } else {
            (self.amount_specified.clone(), None)
        };

        let (_, amount_in, amount_out, fee_amount) = SwapMath::compute_swap_step(
            self.sqrt_price_current_x96,
            self.sqrt_price_limit_x96,
            self.liquidity,
            adjusted_amount,
            self.fee_pips,
        ).unwrap_or_else(|_| (self.sqrt_price_current_x96, U256::zero(), self.amount_specified.abs(), U256::zero()));

        let delta = if self.zero_for_one {
            BalanceDelta {
                amount0: I256::from(amount_in) + I256::from(fee_amount),
                amount1: -I256::from(amount_out),
            }
        } else {
            BalanceDelta {
                amount0: -I256::from(amount_out),
                amount1: I256::from(amount_in) + I256::from(fee_amount),
            }
        };

        let final_delta = if self.hook_data == vec![2] {
            if self.zero_for_one {
                BalanceDelta {
                    amount0: delta.amount0,
                    amount1: delta.amount1 * I256::from(99i128) / I256::from(100i128),
                }
            } else {
                BalanceDelta {
                    amount0: delta.amount0 * I256::from(99i128) / I256::from(100i128),
                    amount1: delta.amount1,
                }
            }
        } else {
            delta
        };

        // Use inner() to access BigInt and convert to 32-byte array
        let amount0_bigint = final_delta.amount0.inner();
        let amount1_bigint = final_delta.amount1.inner();
        let mut amount0_bytes = amount0_bigint.to_signed_bytes_le();
        let mut amount1_bytes = amount1_bigint.to_signed_bytes_le();

        // Pad or truncate to 32 bytes
        amount0_bytes.resize(32, if amount0_bigint.sign() == num_bigint::Sign::Minus { 0xff } else { 0 });
        amount1_bytes.resize(32, if amount1_bigint.sign() == num_bigint::Sign::Minus { 0xff } else { 0 });

        vec![
            U256::from_little_endian(&amount0_bytes[..32]),
            U256::from_little_endian(&amount1_bytes[..32])
        ]
    }
}

impl GKRProver for SwapCircuitGKR {}

pub fn generate_swap_proof(
    zero_for_one: bool,
    amount_specified: I256,
    sqrt_price_limit_x96: U160,
    sqrt_price_current_x96: U160,
    liquidity: u128,
    fee_pips: u32,
    hook_data: Vec<u8>,
) -> Proof {
    let circuit = SwapCircuitGKR {
        zero_for_one,
        amount_specified,
        sqrt_price_limit_x96,
        sqrt_price_current_x96,
        liquidity,
        fee_pips,
        hook_data,
    };
   
    let initial_proof = Proof { bytes: vec![0xAA; 32] };
    let previous_proofs = vec![initial_proof.clone()];
    // Pass the circuit and previous_proofs to generate_gkr_proof, ensuring the initial proof is used as-is
    let (proof, _all_proofs) = generate_gkr_proof(&[&circuit as &dyn GKRProver], &previous_proofs);
    proof
}

pub fn verify_swap_proof(proof: &Proof, previous_proofs: &[Proof]) -> bool {
    verify_gkr_proof(proof, previous_proofs)
}