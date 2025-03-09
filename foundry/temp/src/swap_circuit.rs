use expander_compiler::frontend::{Define, Config, BasicAPI, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use expander_transcript::Proof;
use expander_compiler::field::{BN254};
use crate::proof::{generate_gkr_proof, verify_gkr_proof, u256_to_bn254};
use ark_bn254::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable as R1CSVariable};
use ark_std::{One};
use ark_relations::lc;
use primitive_types::U256;
use arith::FieldForECC;
use ethnum::U256 as EthnumU256;
use ark_ff::PrimeField as ArkPrimeField;


#[derive(Clone)]
pub struct SwapCircuitGKR {
    pub input_token: U256,
    pub output_token: U256,
    pub liquidity: U256,
    pub slippage_tolerance: U256, // In 10^18 format (e.g., 0.01 * 10^18 for 1%)
    pub expected_output: U256,
}

impl<C: Config> Define<C> for SwapCircuitGKR
where
    C::CircuitField: From<BN254> + PartialOrd + Clone + FieldForECC,
{
    fn define(&self, builder: &mut RootBuilder<C>) {
        let input_token = builder.constant(u256_to_bn254(self.input_token));
        let _output_token = builder.constant(u256_to_bn254(self.output_token));
        let liquidity = builder.constant(u256_to_bn254(self.liquidity));
        let slippage_tolerance = builder.constant(u256_to_bn254(self.slippage_tolerance));
        let expected_output = builder.constant(u256_to_bn254(self.expected_output));

        let numerator = builder.mul(input_token, liquidity);
        let denominator = builder.add(liquidity, input_token);
        let actual_output = builder.div(numerator, denominator, false);

        let slippage_diff = builder.sub(actual_output, expected_output);
        let scale = builder.constant(BN254::from_u256(primitive_to_ethnum_u256(U256::from(10).pow(U256::from(18)))));
        let mul_result = builder.mul(slippage_tolerance, expected_output);
        let max_slippage = builder.div(mul_result, scale, false);
        builder.sub(max_slippage, slippage_diff); // no constraints on this value

        // // Assert slack >= 0
        // // If slack >= 0, is_non_negative should be 0 
        // let zero = builder.constant(BN254::zero());
        // let diff = builder.sub(slack, zero);
        // let one = builder.constant(BN254::one());
        // let is_negative = builder.sub(zero, slack); // slack < 0
        // let is_non_negative = builder.is_zero(is_negative);
        // builder.assert_is_equal(is_non_negative, one);        
    }
}

#[derive(Clone)]
pub struct ProofWrapperCircuit {
    pub proof_hash: Vec<u8>,
}

impl ConstraintSynthesizer<Fr> for ProofWrapperCircuit {
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

pub fn generate_swap_proof(
    input_token: U256,
    output_token: U256,
    liquidity: U256,
    slippage_tolerance: U256,
    expected_output: U256,
) -> Proof {
    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(5, 2);
    let circuit = SwapCircuitGKR {
        input_token,
        output_token,
        liquidity,
        slippage_tolerance,
        expected_output,
    };
    circuit.define(&mut api);
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
    generate_gkr_proof(&previous_proofs)
}

pub fn verify_swap_proof<C: Config<CircuitField = BN254>>(
    proof: &Proof,
    input_token: U256,
    output_token: U256,
    liquidity: U256,
    slippage_tolerance: U256,
    expected_output: U256,
    previous_proofs: &[Proof],
) -> bool
where
    C::CircuitField: FieldForECC + PartialOrd,
{
    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(5, 2);
    let circuit = SwapCircuitGKR {
        input_token,
        output_token,
        liquidity,
        slippage_tolerance,
        expected_output,
    };
    circuit.define(&mut api);

    if !verify_gkr_proof(proof, previous_proofs) {
        println!("GKR Proof verification failed!");
        return false;
    }
    
   let scale = U256::from(10).pow(U256::from(18));
   let numerator = input_token * liquidity;
   let denominator = liquidity + input_token;
   let actual_output = if denominator == U256::zero() {
       U256::zero() // handle division by zero
   } else {
        numerator / denominator
   };
   let slippage_diff = if actual_output > expected_output {
       actual_output - expected_output
   } else {
       expected_output - actual_output
   };
   let max_slippage = (slippage_tolerance * expected_output) / scale;

   // Fail if slippage_diff > max_slippage
   if slippage_diff > max_slippage {
       println!("Slippage tolerance exceeded: {} > {}, verification failed!", slippage_diff, max_slippage);
       return false;
   }

    true
}

fn primitive_to_ethnum_u256(u: primitive_types::U256) -> EthnumU256 {
    let mut bytes = [0u8; 32];
    u.to_little_endian(&mut bytes);
    EthnumU256::from_le_bytes(bytes)
}