#[cfg(test)]
mod tests {
    use crate::liquidity_circuit::{LiquidityCircuit, BalanceDelta};
    use crate::proof::{generate_gkr_proof, verify_gkr_proof};
    use crate::libraries::types::{U256, U160, I256};
    use expander_transcript::Proof;
    use expander_compiler::frontend::{API, Define};
    use expander_compiler::circuit::config::BN254Config;

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
        assert!(!proof.bytes.is_empty(), "Liquidity proof should not be empty!");
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
        assert!(is_valid, "Before hook liquidity proof verification failed!");
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
        assert!(is_valid, "After hook liquidity proof verification failed!");
    }

    #[test]
    fn test_liquidity_proof_with_negative_delta() {
        let circuit = LiquidityCircuit {
            owner: U256::from(100),
            tick_lower: 50,
            tick_upper: 100,
            liquidity_delta: -1000, // Negative liquidity delta
            tick_spacing: 10,
            salt: [0u8; 32],
            sqrt_price_current_x96: U256::from(79228162514264337593543950336u128),
            hook_data: vec![1], // Before hook increases absolute value by 1% (to -1010)
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Negative liquidity delta proof verification failed!");
        // Expected: caller_delta amounts are negative, adjusted to -1010 liquidity
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
        assert!(is_valid_insufficient, "Insufficient liquidity proof verification failed!");

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
    fn test_liquidity_proof_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(!is_valid, "Invalid liquidity proof should fail verification!");
    }
}