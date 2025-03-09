use expander_compiler::frontend::{BasicAPI, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use circuit_std_rs::poseidon_m31::*;
use primitive_types::U256;
use arith::FieldForECC;

pub struct MerkleTree {
    leaves: Vec<U256>,
    root: U256,
}

impl MerkleTree {
    pub fn new(leaves: Vec<U256>) -> Self {
        let mut api = RootBuilder::<BN254Config>::new(0, 0).0;
        let poseidon_params = PoseidonM31Params::new(&mut api, POSEIDON_M31X16_RATE, 16, POSEIDON_M31X16_FULL_ROUNDS, POSEIDON_M31X16_PARTIAL_ROUNDS);

        let root = Self::compute_root(&mut api, &poseidon_params, &leaves);
        MerkleTree { leaves, root }
    }

    fn compute_root(api: &mut RootBuilder<BN254Config>, poseidon: &PoseidonM31Params, leaves: &[U256]) -> U256 {
        let mut current_level: Vec<U256> = leaves.to_vec();
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() { current_level[i + 1] } else { left };
                let left_u32 = left.low_u32();
                let right_u32 = right.low_u32();
                let hash_inputs = vec![
                    api.constant(<BN254Config as expander_compiler::frontend::Config>::CircuitField::from(left_u32)),
                    api.constant(<BN254Config as expander_compiler::frontend::Config>::CircuitField::from(right_u32)),
                ];
                let hash = poseidon.hash_to_state(api, &hash_inputs);
                if let Some(hash_value) = api.constant_value(hash[0]) {
                    let ethnum_u256 = hash_value.to_u256();
                    let mut bytes = [0u8; 32];
                    bytes[0..16].copy_from_slice(&ethnum_u256.0[0].to_le_bytes());
                    bytes[16..32].copy_from_slice(&ethnum_u256.0[1].to_le_bytes());
                    next_level.push(U256::from_little_endian(&bytes));
                } else {
                    panic!("Failed to compute hash value");
                }
            }
            current_level = next_level;
        }
        current_level[0]
    }

    pub fn get_root(&self) -> U256 {
        self.root
    }

    pub fn get_proof(&self, leaf_index: usize) -> Vec<U256> {
        let mut api = RootBuilder::<BN254Config>::new(0, 0).0;
        let poseidon_params = PoseidonM31Params::new(&mut api, POSEIDON_M31X16_RATE, 16, POSEIDON_M31X16_FULL_ROUNDS, POSEIDON_M31X16_PARTIAL_ROUNDS);

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
                let left_u32 = left.low_u32();
                let right_u32 = right.low_u32();
                let hash_inputs = vec![
                    api.constant(<BN254Config as expander_compiler::frontend::Config>::CircuitField::from(left_u32)),
                    api.constant(<BN254Config as expander_compiler::frontend::Config>::CircuitField::from(right_u32)),
                ];
                let hash = poseidon_params.hash_to_state(&mut api, &hash_inputs);
                if let Some(hash_value) = api.constant_value(hash[0]) {
                    let ethnum_u256 = hash_value.to_u256();
                    let mut bytes = [0u8; 32];
                    bytes[0..16].copy_from_slice(&ethnum_u256.0[0].to_le_bytes());
                    bytes[16..32].copy_from_slice(&ethnum_u256.0[1].to_le_bytes());
                    next_level.push(U256::from_little_endian(&bytes));
                }
            }
            current_level = next_level;
            index /= 2;
        }
        proof
    }
}