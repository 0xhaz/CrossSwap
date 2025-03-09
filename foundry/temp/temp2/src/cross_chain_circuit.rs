use expander_compiler::frontend::{Define, BasicAPI, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use crate::poseidon_bn254::Poseidon;
use expander_transcript::Proof;
use expander_compiler::field::BN254;
use arith::FieldForECC;
use ark_bn254::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable as R1CSVariable};
use ark_std::One;
use ark_serialize::CanonicalSerialize;
use ark_ff::PrimeField;
use crate::libraries::types::U256;
use crate::proof::{u256_to_bn254, primitive_to_ethnum_u256, CircuitPublicOutputs, GKRProver};
use ethnum::U256 as EthnumU256;
use ark_relations::lc;
use std::any::Any;

#[derive(Clone)]
pub struct CrossChainCircuit {
    pub old_state_root: U256,
    pub new_state_root: U256,
    pub merkle_proof: Vec<U256>,
    pub leaf: U256,
    pub leaf_index: usize,
}

impl Define<BN254Config> for CrossChainCircuit {
    fn define(&self, builder: &mut RootBuilder<BN254Config>) {
        let _old_root = builder.constant(u256_to_bn254(self.old_state_root));
        let new_root = builder.constant(u256_to_bn254(self.new_state_root));
        let mut current = builder.constant(u256_to_bn254(self.leaf));

        let poseidon = Poseidon::new(8, 1, 2);
        let mut level_index = self.leaf_index;

        for sibling in &self.merkle_proof {
            let current_value = builder.constant_value(current).expect("Current value not constant");
            let sibling_value = u256_to_bn254(*sibling);

            let current_bytes = current_value.to_u256().to_le_bytes();
            let current_fr = Fr::from_le_bytes_mod_order(&current_bytes);
            let sibling_bytes = sibling_value.to_u256().to_le_bytes();
            let sibling_fr = Fr::from_le_bytes_mod_order(&sibling_bytes);

            let inputs = if level_index % 2 == 0 {
                vec![current_fr, sibling_fr]
            } else {
                vec![sibling_fr, current_fr]
            };
            let hash_fr = poseidon.hash(&inputs).expect("Poseidon hash failed");

            let mut hash_bytes = [0u8; 32];
            hash_fr.serialize_compressed(&mut hash_bytes[..]).unwrap();
            let hash_u256 = U256::from_little_endian(&hash_bytes);
            let hash_ethnum_u256 = primitive_to_ethnum_u256(hash_u256);
            current = builder.constant(BN254::from_u256(hash_ethnum_u256));

            level_index /= 2;
        }

        let diff_new = builder.sub(new_root, current);
        builder.assert_is_zero(diff_new);
    }
}

impl CircuitPublicOutputs for CrossChainCircuit {
    fn as_any(&self) -> &dyn Any { self }
    fn get_public_outputs(&self) -> Vec<U256> {
        vec![self.old_state_root, self.new_state_root]
    }
}

impl GKRProver for CrossChainCircuit {}

#[derive(Clone)]
pub struct CrossChainProofWrapperCircuit {
    pub proof_hash: Vec<u8>,
}

impl ConstraintSynthesizer<Fr> for CrossChainProofWrapperCircuit {
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