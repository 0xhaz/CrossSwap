#[cfg(test)]
mod tests {
    use crate::cross_chain_circuit::{CrossChainCircuit};
    use crate::proof::{ GKRProver, generate_gkr_proof, verify_gkr_proof };
    use crate::merkle_tree::MerkleTree;
    use crate::libraries::types::U256;
    use expander_transcript::Proof;
    use std::time::Instant;

    fn create_test_circuit(leaf_index: usize, new_leaf_value: u64) -> CrossChainCircuit {
        let old_leaves = vec![U256::from(1), U256::from(2), U256::from(3), U256::from(4)];
        let old_tree = MerkleTree::new(old_leaves.clone());
        let old_state_root = old_tree.get_root();
        let mut new_leaves = old_leaves;
        new_leaves[leaf_index] = U256::from(new_leaf_value);
        let new_tree = MerkleTree::new(new_leaves);
        let new_state_root = new_tree.get_root();
        let leaf = U256::from(new_leaf_value);
        let merkle_proof = new_tree.get_proof(leaf_index);

        CrossChainCircuit {
            old_state_root,
            new_state_root,
            merkle_proof,
            leaf,
            leaf_index,
        }
    }

    #[test]
    fn test_cross_chain_state_root_verification() {
        let circuit = create_test_circuit(0, 5); // Update leaf 0 to 5
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        println!("Single Proof (32 bytes): 0x{}", hex::encode(&proof.bytes));
        println!("Individual Proofs ({} × 32 bytes):", all_proofs.len());
        for (i, p) in all_proofs.iter().enumerate() {
            println!("  Proof {}: 0x{}", i, hex::encode(&p.bytes));
        }

        assert!(!proof.bytes.is_empty(), "GKR proof should not be empty");
        assert!(is_valid, "Single cross-chain proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_cross_chain_batch() {
        // Batch with multiple cross-chain updates
        let circuits = vec![
            create_test_circuit(0, 5),  // Leaf 0 updated to 5
            create_test_circuit(1, 15), // Leaf 1 updated to 15
            create_test_circuit(2, 7),  // Leaf 2 updated to 7
        ];
        let circuits_refs: Vec<&dyn GKRProver> = circuits.iter().map(|c| c as &dyn GKRProver).collect();
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let start = Instant::now();
        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let gen_time = start.elapsed();
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        println!("Batch Proof (32 bytes): 0x{}", hex::encode(&proof.bytes));
        println!("Individual Proofs ({} × 32 bytes):", all_proofs.len());
        for (i, p) in all_proofs.iter().enumerate() {
            println!("  Proof {}: 0x{}", i, hex::encode(&p.bytes));
        }
        println!("Gen Time: {}µs", gen_time.as_micros());

        assert!(!proof.bytes.is_empty(), "Batch GKR proof should not be empty");
        assert!(is_valid, "Batch cross-chain proof verification failed");
        assert_eq!(all_proofs.len(), 3, "Expected 3 individual proofs");
    }

    #[test]
    fn test_cross_chain_alternative_proof() {
        let circuit = create_test_circuit(1, 15); // Update leaf 1 to 15
        let previous_proofs = vec![Proof { bytes: vec![0xBB; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        println!("Alt Proof (32 bytes): 0x{}", hex::encode(&proof.bytes));
        println!("Individual Proofs ({} × 32 bytes):", all_proofs.len());
        for (i, p) in all_proofs.iter().enumerate() {
            println!("  Proof {}: 0x{}", i, hex::encode(&p.bytes));
        }

        assert!(!proof.bytes.is_empty(), "GKR proof should not be empty");
        assert!(is_valid, "Alternative cross-chain proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_cross_chain_proof_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let all_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(!is_valid, "Invalid proof should fail verification");
    }
}