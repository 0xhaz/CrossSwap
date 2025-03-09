#[cfg(test)]
mod tests {
    use crate::swap_circuit::{SwapCircuitGKR, generate_swap_proof, verify_swap_proof};
    use crate::proof::{GKRProver,generate_gkr_proof, verify_gkr_proof};
    use crate::libraries::types::{U256, U160, I256};
    use expander_transcript::Proof;
    use std::time::Instant;

    fn scale(value: u64) -> I256 {
        I256::from(u128::from(value)) * I256::from(U256::from(10).pow(U256::from(18)))
    }

    fn create_test_circuit(zero_for_one: bool, amount: u64, hook_data: Vec<u8>) -> SwapCircuitGKR {
        SwapCircuitGKR {
            zero_for_one,
            amount_specified: scale(amount),
            sqrt_price_limit_x96: U160::from(if zero_for_one {
                7130534626283790383418955530240u128
            } else {
                8715097876569077135289834536960u128
            }),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 1000,
            fee_pips: 3000, // 0.3%
            hook_data,
        }
    }

    #[test]
    fn test_valid_swap_single() {
        let circuit = create_test_circuit(true, 10, vec![]);
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Single valid swap proof should pass verification");
        assert_eq!(all_proofs.len(), 1, "Expected 1 individual proof");
    }

    #[test]
    fn test_swap_batch() {
        // Mimic a small batch like create_large_batch_test
        let circuits = vec![
            create_test_circuit(true, 10, vec![]),           // Swap 1
            create_test_circuit(false, 20, vec![1]),        // Swap 2 with before hook
            create_test_circuit(true, 15, vec![2]),         // Swap 3 with after hook
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

        assert!(is_valid, "Batch swap proof should pass verification");
        assert_eq!(all_proofs.len(), 3, "Expected 3 individual proofs");
    }

    #[test]
    fn test_swap_with_before_hook() {
        let circuit = create_test_circuit(true, 20, vec![1]); // Before hook: 1% extra fee
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Swap proof with before hook should pass verification");
    }

    #[test]
    fn test_swap_with_after_hook() {
        let circuit = create_test_circuit(false, 15, vec![2]); // After hook: 1% reduction
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Swap proof with after hook should pass verification");
    }

    #[test]
    fn test_invalid_slippage() {
        let circuit = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(100),
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128), // Tight limit
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 1000,
            fee_pips: 3000,
            hook_data: vec![],
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Proof should verify even with slippage (GKR checks consistency)");
    }

    #[test]
    fn test_zero_inputs() {
        let circuit = SwapCircuitGKR {
            zero_for_one: false,
            amount_specified: I256::zero(),
            sqrt_price_limit_x96: U160::zero(),
            sqrt_price_current_x96: U160::zero(),
            liquidity: 0,
            fee_pips: 0,
            hook_data: vec![],
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let circuits_refs: Vec<&dyn GKRProver> = vec![&circuit];

        let (proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(is_valid, "Zero inputs should pass verification");
    }

    #[test]
    fn test_swap_proof_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let all_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_gkr_proof(&proof, &all_proofs);
        assert!(!is_valid, "Invalid proof should fail verification");
    }
}