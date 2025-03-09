use expander_compiler::frontend::{API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use crate::poseidon_bn254::Poseidon; 
use crate::libraries::types::U256;
use arith::FieldForECC;
use ark_bn254::Fr;
use ark_serialize::CanonicalSerialize;
use ark_ff::PrimeField;

pub struct MerkleTree {
    leaves: Vec<U256>, // types::U256
    root: U256,       // types::U256
}

impl MerkleTree {
    pub fn new(leaves: Vec<U256>) -> Self { // types::U256
        let mut api = RootBuilder::<BN254Config>::new(0, 0).0;
        let poseidon = Poseidon::new(8, 1, 2); // Use poseidon_bn254

        let root = Self::compute_root(&mut api, &poseidon, &leaves);
        MerkleTree { leaves, root }
    }

    fn compute_root(_api: &mut RootBuilder<BN254Config>, poseidon: &Poseidon, leaves: &[U256]) -> U256 {
        let mut current_level: Vec<U256> = leaves.to_vec();
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() { current_level[i + 1] } else { left };

                // Convert U256 to Fr for Poseidon hashing
                let mut left_bytes = [0u8; 32];
                let mut right_bytes = [0u8; 32];
                left.to_little_endian(&mut left_bytes);
                right.to_little_endian(&mut right_bytes);
                let left_fr = Fr::from_le_bytes_mod_order(&left_bytes);
                let right_fr = Fr::from_le_bytes_mod_order(&right_bytes);

                let hash_inputs = vec![left_fr, right_fr];
                let hash_fr = poseidon.hash(&hash_inputs).expect("Poseidon hash failed");
                let mut hash_bytes = [0u8; 32];
                hash_fr.serialize_compressed(&mut hash_bytes[..]).unwrap();
                let hash_u256 = U256::from_little_endian(&hash_bytes);

                next_level.push(hash_u256);
            }
            current_level = next_level;
        }
        current_level[0]
    }

    pub fn get_root(&self) -> U256 { // types::U256
        self.root
    }

    pub fn get_proof(&self, leaf_index: usize) -> Vec<U256> { // types::U256
        let mut _api = RootBuilder::<BN254Config>::new(0, 0).0;
        let poseidon = Poseidon::new(8, 1, 2);

        let mut proof = Vec::new();
        let mut current_level = self.leaves.clone();
        let mut index = leaf_index;

        while current_level.len() > 1 {
            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            let sibling = if sibling_index < current_level.len() { current_level[sibling_index] } else { current_level[index] };
            proof.push(sibling);

            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() { current_level[i + 1] } else { left };

                let mut left_bytes = [0u8; 32];
                let mut right_bytes = [0u8; 32];
                left.to_little_endian(&mut left_bytes);
                right.to_little_endian(&mut right_bytes);
                let left_fr = Fr::from_le_bytes_mod_order(&left_bytes);
                let right_fr = Fr::from_le_bytes_mod_order(&right_bytes);

                let hash_inputs = vec![left_fr, right_fr];
                let hash_fr = poseidon.hash(&hash_inputs).expect("Poseidon hash failed");
                let mut hash_bytes = [0u8; 32];
                hash_fr.serialize_compressed(&mut hash_bytes[..]).unwrap();
                let hash_u256 = U256::from_little_endian(&hash_bytes);

                next_level.push(hash_u256);
            }
            current_level = next_level;
            index /= 2;
        }
        proof
    }
}