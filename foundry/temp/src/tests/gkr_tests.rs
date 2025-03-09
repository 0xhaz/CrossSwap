#[cfg(test)]
mod tests {
    use crate::proof::{generate_gkr_proof, verify_gkr_proof, generate_proof, verify_proof};
    use crate::liquidity_circuit::{generate_liquidity_proof, verify_liquidity_proof};
    use crate::swap_circuit::SwapCircuitGKR; 
    use expander_compiler::frontend::{API, Define};
    use expander_compiler::circuit::config::BN254Config;
    use expander_transcript::Proof;
    use primitive_types::U256;

    #[test]
    fn test_gkr_proof_generation() {
        let proofs = vec![];
        let proof = generate_gkr_proof(&proofs);
        assert!(!proof.bytes.is_empty(), "❌ Generated proof should not be empty");
    }

    #[test]
    fn test_gkr_proof_verification() {
        let proofs = vec![];
        let proof = generate_gkr_proof(&proofs);
        let is_valid = verify_gkr_proof(&proof, &proofs);
        assert!(is_valid, "❌ GKR Proof verification failed!");
    }

    #[test]
    fn test_gkr_empty_proof() {
        let proof = Proof { bytes: vec![] };
        let is_valid = verify_gkr_proof(&proof, &[]);
        assert!(!is_valid, "❌ Empty proof should fail verification");
    }

    #[test]
    fn test_liquidity_proof_generation() {
        let proof = generate_liquidity_proof(U256::from(100), U256::from(50), U256::from(1000), U256::from(1050));
        assert!(!proof.bytes.is_empty(), "❌ Liquidity proof should not be empty");
    }
    
    #[test]
    fn test_liquidity_proof_verification() {
        let proof = generate_liquidity_proof(U256::from(100), U256::from(50), U256::from(1000), U256::from(1050));
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_liquidity_proof::<BN254Config>(&proof, U256::from(100), U256::from(50), U256::from(1000), U256::from(1050), &previous_proofs);
        assert!(is_valid, "❌ Liquidity proof verification failed!");
    }
    
    #[test]
    fn test_liquidity_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_liquidity_proof::<BN254Config>(&proof, U256::from(100), U256::from(50), U256::from(1000), U256::from(1050), &previous_proofs);
        assert!(!is_valid, "❌ Invalid liquidity proof should fail");
    }

    #[test]
    fn test_liquidity_proof_edge_cases() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof_insufficient = generate_liquidity_proof(10.into(), 20.into(), 1000.into(), 1020.into());
        let is_valid_insufficient = verify_liquidity_proof::<BN254Config>(
            &proof_insufficient,
            10.into(), 20.into(), 1000.into(), 1020.into(),
            &previous_proofs,
        );
        assert!(!is_valid_insufficient);
        let proof_zero = generate_liquidity_proof(0.into(), 0.into(), 0.into(), 0.into());
        let is_valid_zero = verify_liquidity_proof::<BN254Config>(
            &proof_zero,
            0.into(), 0.into(), 0.into(), 0.into(),
            &previous_proofs,
        );
        assert!(is_valid_zero);
    }

    #[test]
    fn test_hybrid_proof_swap() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let (gkr_proof, solidity_proof, public_inputs, vk) = generate_proof(
            0, // Swap
            U256::from(1000), // input_token
            U256::from(900),  // output_token
            U256::from(500),  // liquidity
            U256::from(5),    // slippage_tolerance
            U256::from(2500), // expected_output
            U256::zero(),     // user_balance (unused)
            U256::zero(),     // pool_total_liquidity (unused)
            &previous_proofs,
            U256::zero(),     // old_state_root (unused)
            U256::zero(),     // new_state_root (unused)
            vec![],           // merkle_proof (unused)
            U256::zero()      // leaf (unused)
        );
        assert!(!gkr_proof.bytes.is_empty(), "❌ GKR proof should not be empty");
        assert_eq!(solidity_proof.len(), 3, "❌ Solidity proof should have 3 components");
        assert_eq!(public_inputs.len(), 1, "❌ Expected single public input (GKR proof hash)");
        let is_valid = verify_proof(&gkr_proof, solidity_proof, public_inputs, &vk, &previous_proofs);
        assert!(is_valid, "❌ Hybrid swap proof verification failed!");
    }

    #[test]
    fn test_hybrid_proof_liquidity() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let (gkr_proof, solidity_proof, public_inputs, vk) = generate_proof(
            1, // Liquidity
            U256::zero(),     // input_token (unused)
            U256::zero(),     // output_token (unused)
            U256::from(500),  // liquidity_added
            U256::zero(),     // slippage_tolerance (unused)
            U256::from(2500), // expected_new_total
            U256::from(1000), // user_balance
            U256::from(2000), // pool_total_liquidity
            &previous_proofs,
            U256::zero(),     // old_state_root (unused)
            U256::zero(),     // new_state_root (unused)
            vec![],           // merkle_proof (unused)
            U256::zero()      // leaf (unused)
        );
        assert!(!gkr_proof.bytes.is_empty(), "❌ GKR proof should not be empty");
        assert_eq!(solidity_proof.len(), 3, "❌ Solidity proof should have 3 components");
        assert_eq!(public_inputs.len(), 1, "❌ Expected single public input (GKR proof hash)");
        let is_valid = verify_proof(&gkr_proof, solidity_proof, public_inputs, &vk, &previous_proofs);
        assert!(is_valid, "❌ Hybrid liquidity proof verification failed!");
    }

    #[test]
    fn test_swap_circuit_gkr_proof() {
        let circuit = SwapCircuitGKR {
            input_token: U256::from(1000),
            output_token: U256::from(900),
            liquidity: U256::from(500),
            slippage_tolerance: U256::from(5),
            expected_output: U256::from(2500),
        };
        let (mut api, _input_vars, _public_vars) = API::<BN254Config>::new(5, 0);
        circuit.define(&mut api);
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&previous_proofs);
        assert!(!proof.bytes.is_empty(), "❌ GKR proof from swap circuit should not be empty");
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "❌ GKR proof verification for swap circuit failed!");
    }
}