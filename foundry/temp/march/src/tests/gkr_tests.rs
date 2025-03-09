#[cfg(test)]
mod tests {
    use crate::cross_chain_circuit::CrossChainCircuit;
    use crate::liquidity_circuit::LiquidityCircuit;
    use crate::merkle_tree::MerkleTree;
    use crate::proof::{generate_gkr_proof, verify_gkr_proof};
    use crate::swap_circuit::SwapCircuitGKR;
    use crate::libraries::types::{U256, U160, I256};
    use expander_transcript::Proof;

    fn scale(value: u64) -> I256 {
        I256::from(u128::from(value)) * I256::from(U256::from(10).pow(U256::from(18)))
    }

    #[test]
    fn test_gkr_proof_empty_previous_proofs() {
        let circuit = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(1000), // 1000 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 500,
            fee_pips: 3000,
            hook_data: vec![], // No hook
        };
        let proofs = vec![];
        let proof = generate_gkr_proof(&circuit, &proofs);
        assert!(!proof.bytes.is_empty(), "Generated proof should not be empty");
        let is_valid = verify_gkr_proof(&proof, &proofs);
        assert!(is_valid, "GKR Proof verification failed with empty previous proofs!");
    }

    #[test]
    fn test_gkr_proof_verification() {
        let circuit = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(1000), // 1000 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 500,
            fee_pips: 3000,
            hook_data: vec![], // No hook
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        assert!(!proof.bytes.is_empty(), "Generated proof should not be empty");
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "GKR Proof verification failed!");
    }

    #[test]
    fn test_gkr_empty_proof() {
        let proof = Proof { bytes: vec![] };
        let is_valid = verify_gkr_proof(&proof, &[]);
        assert!(!is_valid, "Empty proof should fail verification");
    }

    #[test]
    fn test_liquidity_proof_generation() {
        let circuit = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: 1000,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![], // No hook
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        assert!(!proof.bytes.is_empty(), "Liquidity proof should not be empty");
    }

    #[test]
    fn test_liquidity_proof_verification() {
        let circuit = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: 1000,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![], // No hook
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Liquidity proof verification failed!");
    }

    #[test]
    fn test_liquidity_proof_with_before_hook() {
        let circuit = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: 1000,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![1], // Before hook increases liquidity_delta by 1%
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Liquidity proof with before hook verification failed!");
    }

    #[test]
    fn test_liquidity_proof_with_after_hook() {
        let circuit = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: 1000,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![2], // After hook reduces caller_delta by 1%
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Liquidity proof with after hook verification failed!");
    }

    #[test]
    fn test_liquidity_proof_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(!is_valid, "Invalid liquidity proof should fail");
    }

    #[test]
    fn test_liquidity_proof_edge_cases() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        // Insufficient liquidity delta
        let circuit_insufficient = LiquidityCircuit {
            owner: U256::from(10),
            tick_lower: 20,
            tick_upper: 1000,
            liquidity_delta: 10,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![], // No hook
        };
        let proof_insufficient = generate_gkr_proof(&circuit_insufficient, &previous_proofs);
        let is_valid_insufficient = verify_gkr_proof(&proof_insufficient, &previous_proofs);
        assert!(is_valid_insufficient, "Low liquidity delta proof verification failed!");

        // Zero values
        let circuit_zero = LiquidityCircuit {
            owner: U256::zero(),
            tick_lower: 0,
            tick_upper: 0,
            liquidity_delta: 0,
            tick_spacing: 1,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::zero(),
            hook_data: vec![], // No hook
        };
        let proof_zero = generate_gkr_proof(&circuit_zero, &previous_proofs);
        let is_valid_zero = verify_gkr_proof(&proof_zero, &previous_proofs);
        assert!(is_valid_zero, "Zero liquidity proof verification failed!");
    }

    #[test]
    fn test_swap_circuit_gkr_proof() {
        let circuit = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(1000), // 1000 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 500,
            fee_pips: 3000,
            hook_data: vec![], // No hook
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        assert!(!proof.bytes.is_empty(), "GKR proof from swap circuit should not be empty");
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "GKR proof verification for swap circuit failed!");
    }

    #[test]
    fn test_swap_circuit_gkr_proof_with_hooks() {
        let circuit_before = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(1000), // 1000 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 500,
            fee_pips: 3000,
            hook_data: vec![1], // Before hook adds 1% fee
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof_before = generate_gkr_proof(&circuit_before, &previous_proofs);
        let is_valid_before = verify_gkr_proof(&proof_before, &previous_proofs);
        assert!(is_valid_before, "GKR proof with before hook for swap circuit failed!");

        let circuit_after = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(1000), // 1000 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 500,
            fee_pips: 3000,
            hook_data: vec![2], // After hook reduces amount_out by 1%
        };
        let proof_after = generate_gkr_proof(&circuit_after, &previous_proofs);
        let is_valid_after = verify_gkr_proof(&proof_after, &previous_proofs);
        assert!(is_valid_after, "GKR proof with after hook for swap circuit failed!");
    }

    #[test]
    fn test_cross_chain_proof() {
        let old_leaves = vec![U256::from(1), U256::from(2), U256::from(3), U256::from(4)];
        let old_tree = MerkleTree::new(old_leaves.clone());
        let old_state_root = old_tree.get_root();
        let mut new_leaves = old_leaves;
        new_leaves[0] = U256::from(5);
        let new_tree = MerkleTree::new(new_leaves);
        let new_state_root = new_tree.get_root();
        let leaf = U256::from(5);
        let merkle_proof = new_tree.get_proof(0);

        let circuit = CrossChainCircuit {
            old_state_root,
            new_state_root,
            merkle_proof,
            leaf,
            leaf_index: 0,
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        assert!(!proof.bytes.is_empty(), "CrossChain proof should not be empty");
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "CrossChain proof verification failed!");
    }

    #[test]
    fn test_liquidity_proof_negative_delta() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

        // No hook
        let circuit_no_hook = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: -1000,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![], // No hook
        };
        let proof_no_hook = generate_gkr_proof(&circuit_no_hook, &previous_proofs);
        let is_valid_no_hook = verify_gkr_proof(&proof_no_hook, &previous_proofs);
        assert!(is_valid_no_hook, "Negative liquidity delta proof (no hook) verification failed!");

        // Before hook (increases absolute liquidity_delta by 1%)
        let circuit_before = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: -1000,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![1], // Before hook
        };
        let proof_before = generate_gkr_proof(&circuit_before, &previous_proofs);
        let is_valid_before = verify_gkr_proof(&proof_before, &previous_proofs);
        assert!(is_valid_before, "Negative liquidity delta proof (before hook) verification failed!");

        // After hook (reduces caller_delta by 1%)
        let circuit_after = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: -1000,
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![2], // After hook
        };
        let proof_after = generate_gkr_proof(&circuit_after, &previous_proofs);
        let is_valid_after = verify_gkr_proof(&proof_after, &previous_proofs);
        assert!(is_valid_after, "Negative liquidity delta proof (after hook) verification failed!");
    }
}