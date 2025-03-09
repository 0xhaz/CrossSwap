#[cfg(test)]
mod tests {
    use crate::cross_chain_circuit::CrossChainCircuit;
    use crate::merkle_tree::MerkleTree;
    use crate::proof::{generate_gkr_proof, verify_gkr_proof};
    use crate::libraries::types::U256;
    use expander_transcript::Proof;
    use expander_compiler::frontend::{API, Define};
    use expander_compiler::circuit::config::BN254Config;

    #[test]
    fn test_cross_chain_state_root_verification() {
        let (mut api, _, _) = API::<BN254Config>::new(0, 0);

        let old_leaves = vec![U256::from(1), U256::from(2), U256::from(3), U256::from(4)];
        let old_tree = MerkleTree::new(old_leaves.clone());
        let old_state_root = old_tree.get_root();
        let mut new_leaves = old_leaves;
        new_leaves[0] = U256::from(5);
        let new_tree = MerkleTree::new(new_leaves);
        let new_state_root = new_tree.get_root();
        let leaf = U256::from(5);
        let merkle_proof = new_tree.get_proof(0);

        println!("Test 1 - Old Root: {:?}", old_state_root);
        println!("Test 1 - New Root: {:?}", new_state_root);
        println!("Test 1 - Leaf: {:?}", leaf);
        println!("Test 1 - Merkle Proof: {:?}", merkle_proof);

        let circuit = CrossChainCircuit {
            old_state_root,
            new_state_root,
            merkle_proof,
            leaf,
            leaf_index: 0,
        };
        circuit.define(&mut api);

        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let gkr_proof = generate_gkr_proof(&circuit, &previous_proofs);
        assert!(!gkr_proof.bytes.is_empty(), "❌ GKR proof for cross-chain should not be empty");
        assert!(verify_gkr_proof(&gkr_proof, &previous_proofs), "❌ GKR proof verification failed");
    }

    #[test]
    fn test_cross_chain_alternative_proof() {
        let (mut api, _, _) = API::<BN254Config>::new(0, 0);

        let old_leaves = vec![U256::from(10), U256::from(3), U256::from(7), U256::from(8)];
        let old_tree = MerkleTree::new(old_leaves.clone());
        let old_state_root = old_tree.get_root();
        let mut new_leaves = old_leaves;
        new_leaves[1] = U256::from(15);
        let new_tree = MerkleTree::new(new_leaves);
        let new_state_root = new_tree.get_root();
        let leaf = U256::from(15);
        let merkle_proof = new_tree.get_proof(1);

        println!("Test 2 - Old Root: {:?}", old_state_root);
        println!("Test 2 - New Root: {:?}", new_state_root);
        println!("Test 2 - Leaf: {:?}", leaf);
        println!("Test 2 - Merkle Proof: {:?}", merkle_proof);

        let circuit = CrossChainCircuit {
            old_state_root,
            new_state_root,
            merkle_proof,
            leaf,
            leaf_index: 1,
        };
        circuit.define(&mut api);

        let previous_proofs = vec![Proof { bytes: vec![0xBB; 32] }];
        let gkr_proof = generate_gkr_proof(&circuit, &previous_proofs);
        assert!(!gkr_proof.bytes.is_empty(), "GKR proof for cross-chain should not be empty");
        assert!(verify_gkr_proof(&gkr_proof, &previous_proofs), "GKR proof verification failed");
    }
}