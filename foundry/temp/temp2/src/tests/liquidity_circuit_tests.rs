#[cfg(test)]
mod tests {
    use crate::liquidity_circuit::{LiquidityCircuit, generate_liquidity_proof, verify_liquidity_proof};
    use crate::proof::{GKRProver,generate_gkr_proof, verify_gkr_proof};
    use crate::libraries::types::{U256, I256};
    use expander_transcript::Proof;
    use std::time::Instant;

    fn create_test_circuit(liquidity_delta: i128, hook_data: Vec<u8>) -> LiquidityCircuit {
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

    #[test]
    fn test_liquidity_proof_generation() {
        let circuit = create_test_circuit(1000, vec![]); // No hook
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        
        println!("Single Proof (32 bytes): 0x{}", hex::encode(&proof.bytes));
        println!("Individual Proofs ({} × 32 bytes):", all_proofs.len());
        for (i, p) in all_proofs.iter().enumerate() {
            println!("  Proof {}: 0x{}", i, hex::encode(&p.bytes));
        }

        assert!(!proof.bytes.is_empty(), "Liquidity proof should not be empty");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_verification() {
        let circuit = create_test_circuit(1000, vec![]); // No hook
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        assert!(is_valid, "Liquidity proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_batch() {
        // Batch with multiple liquidity circuits
        let circuits = vec![
            create_test_circuit(1000, vec![]),      // No hook
            create_test_circuit(2000, vec![1]),     // Before hook: +1%
            create_test_circuit(-1500, vec![2]),    // After hook: -1%, negative delta
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

        assert!(!proof.bytes.is_empty(), "Batch liquidity proof should not be empty");
        assert!(is_valid, "Batch liquidity proof verification failed");
        assert_eq!(all_proofs.len(), 3, "Expected 3 individual proofs");
    }

    #[test]
    fn test_liquidity_proof_with_before_hook() {
        let circuit = create_test_circuit(1000, vec![1]); // Before hook: +1%
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        assert!(is_valid, "Before hook liquidity proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_with_after_hook() {
        let circuit = create_test_circuit(1000, vec![2]); // After hook: -1%
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        assert!(is_valid, "After hook liquidity proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_liquidity_proof_with_negative_delta() {
        let circuit = create_test_circuit(-1000, vec![1]); // Before hook: -1010 after adjustment
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);

        assert!(is_valid, "Negative liquidity delta proof verification failed");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
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
            hook_data: vec![],
        };
        let circuits_insufficient: Vec<&dyn GKRProver> = vec![&circuit_insufficient];
        let (proof_insufficient, all_proofs_insufficient) = generate_gkr_proof(&circuits_insufficient, &previous_proofs);
        let is_valid_insufficient = verify_gkr_proof(&proof_insufficient, &all_proofs_insufficient);
        assert!(is_valid_insufficient, "Insufficient liquidity proof verification failed");
        assert_eq!(all_proofs_insufficient.len(), 1, "Expected 1 individual proof");

        // Zero values
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
    fn test_liquidity_proof_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let all_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(!is_valid, "Invalid liquidity proof should fail verification");
    }
}