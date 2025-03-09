#[cfg(test)]
mod tests {
    use crate::proof::{GKRProver, generate_gkr_proof, verify_gkr_proof};
    use crate::swap_circuit::SwapCircuitGKR;
    use crate::liquidity_circuit::LiquidityCircuit;
    use crate::cross_chain_circuit::CrossChainCircuit;
    use crate::merkle_tree::MerkleTree;
    use crate::libraries::types::{U256, U160, I256};
    use expander_transcript::Proof;
    use std::time::Instant;

    fn scale(value: u64) -> I256 {
        I256::from(u128::from(value)) * I256::from(U256::from(10).pow(U256::from(18)))
    }

    fn create_swap_circuit(hook_data: Vec<u8>) -> SwapCircuitGKR {
        SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(1000), // 1000 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 500,
            fee_pips: 3000,
            hook_data,
        }
    }

    fn create_liquidity_circuit(liquidity_delta: i128, hook_data: Vec<u8>) -> LiquidityCircuit {
        LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data,
        }
    }

    fn create_cross_chain_circuit(leaf_index: usize, new_leaf_value: u64) -> CrossChainCircuit {
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
    fn test_gkr_proof_empty_previous_proofs() {
        let circuit = create_swap_circuit(vec![]);
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        println!("Single Proof (empty prev, 32 bytes): 0x{}", hex::encode(&proof.bytes));
        println!("Individual Proofs ({} × 32 bytes):", all_proofs.len());
        for (i, p) in all_proofs.iter().enumerate() {
            println!("  Proof {}: 0x{}", i, hex::encode(&p.bytes));
        }

        assert!(!proof.bytes.is_empty(), "Generated proof should not be empty");
        assert!(is_valid, "GKR proof with empty previous proofs failed verification");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_gkr_proof_verification() {
        let circuit = create_swap_circuit(vec![]);
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        assert!(!proof.bytes.is_empty(), "Generated proof should not be empty");
        assert!(is_valid, "GKR proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_gkr_empty_proof() {
        let proof = Proof { bytes: vec![] };
        let all_proofs = vec![];
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(!is_valid, "Empty proof should fail verification");
    }

    #[test]
    fn test_gkr_batch_proof() {
        // Batch with swap, liquidity, and cross-chain circuits
        let circuits: Vec<Box<dyn GKRProver>> = vec![
            Box::new(create_swap_circuit(vec![])),           // Swap, no hook
            Box::new(create_liquidity_circuit(1000, vec![1])), // Liquidity, before hook
            Box::new(create_cross_chain_circuit(0, 5)),      // Cross-chain, leaf 0 to 5
        ];
        let circuits_refs: Vec<&dyn GKRProver> = circuits.iter().map(|c| c.as_ref()).collect();
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

        assert!(!proof.bytes.is_empty(), "Batch proof should not be empty");
        assert!(is_valid, "Batch GKR proof verification failed");
        assert_eq!(all_proofs.len(), 3, "Expected 3 individual proofs");
    }

    #[test]
    fn test_liquidity_proof_generation() {
        let circuit = create_liquidity_circuit(1000, vec![]);
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        assert!(!proof.bytes.is_empty(), "Liquidity proof should not be empty");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_verification() {
        let circuit = create_liquidity_circuit(1000, vec![]);
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Liquidity proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_with_before_hook() {
        let circuit = create_liquidity_circuit(1000, vec![1]); // Before hook: +1%
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Liquidity proof with before hook verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_with_after_hook() {
        let circuit = create_liquidity_circuit(1000, vec![2]); // After hook: -1%
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Liquidity proof with after hook verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let all_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(!is_valid, "Invalid liquidity proof should fail verification");
    }

    #[test]
    fn test_liquidity_proof_edge_cases() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let circuit_insufficient = LiquidityCircuit {
            owner: U256::from(10),
            tick_lower: 20,
            tick_upper: 1000,
            liquidity_delta: 10,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![],
        };
        let circuits_insufficient: Vec<&dyn GKRProver> = vec![&circuit_insufficient];
        let (proof_insufficient, all_proofs_insufficient) = generate_gkr_proof(&circuits_insufficient, &previous_proofs);
        let is_valid_insufficient = verify_gkr_proof(&proof_insufficient, &all_proofs_insufficient);
        assert!(is_valid_insufficient, "Low liquidity delta proof verification failed");
        assert_eq!(all_proofs_insufficient.len(), 1, "Expected 1 individual proof");

        let circuit_zero = LiquidityCircuit {
            owner: U256::zero(),
            tick_lower: 0,
            tick_upper: 0,
            liquidity_delta: 0,
            tick_spacing: 1,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::zero(),
            hook_data: vec![],
        };
        let circuits_zero: Vec<&dyn GKRProver> = vec![&circuit_zero];
        let (proof_zero, all_proofs_zero) = generate_gkr_proof(&circuits_zero, &previous_proofs);
        let is_valid_zero = verify_gkr_proof(&proof_zero, &all_proofs_zero);
        assert!(is_valid_zero, "Zero liquidity proof verification failed");
        assert_eq!(all_proofs_zero.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_swap_circuit_gkr_proof() {
        let circuit = create_swap_circuit(vec![]);
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(!proof.bytes.is_empty(), "Swap circuit proof should not be empty");
        assert!(is_valid, "Swap circuit GKR proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_swap_circuit_gkr_proof_with_hooks() {
        let circuit_before = create_swap_circuit(vec![1]); // Before hook: +1% fee
        let circuits_before: Vec<&dyn GKRProver> = vec![&circuit_before];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let (proof_before, all_proofs_before) = generate_gkr_proof(&circuits_before, &previous_proofs);
        let is_valid_before = verify_gkr_proof(&proof_before, &all_proofs_before);
        assert!(is_valid_before, "Swap proof with before hook failed verification");
        assert_eq!(all_proofs_before.len(), 1, "Expected 1 individual proof");

        let circuit_after = create_swap_circuit(vec![2]); // After hook: -1% amount_out
        let circuits_after: Vec<&dyn GKRProver> = vec![&circuit_after];
        let (proof_after, all_proofs_after) = generate_gkr_proof(&circuits_after, &previous_proofs);
        let is_valid_after = verify_gkr_proof(&proof_after, &all_proofs_after);
        assert!(is_valid_after, "Swap proof with after hook failed verification");
        assert_eq!(all_proofs_after.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_cross_chain_proof() {
        let circuit = create_cross_chain_circuit(0, 5); // Leaf 0 to 5
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(!proof.bytes.is_empty(), "Cross-chain proof should not be empty");
        assert!(is_valid, "Cross-chain proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_negative_delta() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        let circuit_no_hook = create_liquidity_circuit(-1000, vec![]);
        let circuits_no_hook: Vec<&dyn GKRProver> = vec![&circuit_no_hook];
        let (proof_no_hook, all_proofs_no_hook) = generate_gkr_proof(&circuits_no_hook, &previous_proofs);
        let is_valid_no_hook = verify_gkr_proof(&proof_no_hook, &all_proofs_no_hook);
        assert!(is_valid_no_hook, "Negative liquidity delta proof (no hook) failed");
        assert_eq!(all_proofs_no_hook.len(), 1, "Expected 1 individual proof");

        let circuit_before = create_liquidity_circuit(-1000, vec![1]); // Before hook: -1010
        let circuits_before: Vec<&dyn GKRProver> = vec![&circuit_before];
        let (proof_before, all_proofs_before) = generate_gkr_proof(&circuits_before, &previous_proofs);
        let is_valid_before = verify_gkr_proof(&proof_before, &all_proofs_before);
        assert!(is_valid_before, "Negative liquidity delta proof (before hook) failed");
        assert_eq!(all_proofs_before.len(), 1, "Expected 1 individual proof");

        let circuit_after = create_liquidity_circuit(-1000, vec![2]); // After hook: -1%
        let circuits_after: Vec<&dyn GKRProver> = vec![&circuit_after];
        let (proof_after, all_proofs_after) = generate_gkr_proof(&circuits_after, &previous_proofs);
        let is_valid_after = verify_gkr_proof(&proof_after, &all_proofs_after);
        assert!(is_valid_after, "Negative liquidity delta proof (after hook) failed");
        assert_eq!(all_proofs_after.len(), 1, "Expected 1 individual proof");
    }
}