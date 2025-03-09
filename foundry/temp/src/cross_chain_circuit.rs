use expander_compiler::frontend::{Define, Variable, BasicAPI, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use crate::poseidon_bn254::Poseidon;
use expander_compiler::field::{BN254};
use arith::FieldForECC;
use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable as R1CSVariable};
use ark_relations::lc;
use ark_std::One;
use ark_serialize::{CanonicalSerialize};
use primitive_types::U256 as PrimitiveU256;

pub struct CrossChainCircuit {
    pub old_state_root: Variable,
    pub new_state_root: Variable,
    pub merkle_proof: Vec<Variable>,
    pub leaf: Variable,
}

impl Define<BN254Config> for CrossChainCircuit {
    fn define(&self, builder: &mut RootBuilder<BN254Config>) {
        let old_root = self.old_state_root;
        let new_root = self.new_state_root;
        let mut current = self.leaf;

        let poseidon = Poseidon::new(8, 1, 2);
        for sibling in &self.merkle_proof {
            let current_value = builder.constant_value(current).expect("Current value not constant");
            let sibling_value = builder.constant_value(*sibling).expect("Sibling value not constant");

            let current_bytes = current_value.to_u256().to_le_bytes();
            let current_fr = Fr::from_le_bytes_mod_order(&current_bytes);
            let sibling_bytes = sibling_value.to_u256().to_le_bytes();
            let sibling_fr = Fr::from_le_bytes_mod_order(&sibling_bytes);

            // Revert to original order to match main.rs
            let inputs = vec![current_fr, sibling_fr];
            let hash_fr = poseidon.hash(&inputs).expect("Poseidon hash failed");

            let mut hash_bytes = [0u8; 32];
            hash_fr.serialize_compressed(&mut hash_bytes[..]).unwrap();
            let _hash_u256 = PrimitiveU256::from_little_endian(&hash_bytes);
            let hash_ethnum = ethnum::U256::from_le_bytes(hash_bytes);
            let hash_bn254 = BN254::from_u256(hash_ethnum);

            current = builder.constant(hash_bn254);
        }

        if let Some(root_value) = builder.constant_value(current) {
            println!("🔹 Computed root value: {:?}", root_value.to_u256());
        }
        println!("🔹 Old root: {:?}", old_root);
        println!("🔹 New root: {:?}", new_root);
        println!("🔹 Computed root: {:?}", current);

        let diff_new = builder.sub(new_root, current);
        builder.assert_is_zero(diff_new);
    }
}

#[derive(Clone)]
pub struct CrossChainProofWrapperCircuit {
    pub proof_hash: Vec<u8>,
}

impl ConstraintSynthesizer<Fr> for CrossChainProofWrapperCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let mut proof_hash_bytes = [0u8; 32];
        proof_hash_bytes.copy_from_slice(&self.proof_hash);
        let proof_hash_u256 = PrimitiveU256::from_little_endian(&proof_hash_bytes);
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