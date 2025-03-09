#[cfg(test)]
mod tests {
    use crate::proof::{generate_proof, verify_proof};
    use crate::liquidity_circuit::{generate_liquidity_proof, verify_liquidity_proof};
    use expander_transcript::Proof;
    use primitive_types::U256;
    use expander_compiler::circuit::config::BN254Config;

    #[test]
    fn test_liquidity_proof_generation() {
        let user_balance = U256::from(100);
        let liquidity_added = U256::from(50);
        let pool_total_liquidity = U256::from(1000);
        let expected_new_total = pool_total_liquidity + liquidity_added;

        let proof = generate_liquidity_proof(user_balance, liquidity_added, pool_total_liquidity, expected_new_total);
        assert!(!proof.bytes.is_empty(), "❌ Liquidity proof should not be empty!");
    }

    #[test]
    fn test_liquidity_proof_verification() {
        let user_balance = U256::from(100);
        let liquidity_added = U256::from(50);
        let pool_total_liquidity = U256::from(1000);
        let expected_new_total = pool_total_liquidity + liquidity_added;

        let proof = generate_liquidity_proof(user_balance, liquidity_added, pool_total_liquidity, expected_new_total);
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_liquidity_proof::<BN254Config>(&proof, user_balance, liquidity_added, pool_total_liquidity, expected_new_total, &previous_proofs);
        assert!(is_valid, "❌ Liquidity proof verification failed!");
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
    fn test_hybrid_liquidity_proof() {
        let user_balance = U256::from(1000);
        let liquidity_added = U256::from(500);
        let pool_total_liquidity = U256::from(2000);
        let expected_new_total = pool_total_liquidity + liquidity_added;

        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let (gkr_proof, solidity_proof, public_inputs, vk) = generate_proof(
            1, U256::zero(), U256::zero(), liquidity_added, U256::zero(), expected_new_total, user_balance, pool_total_liquidity,
            &previous_proofs, U256::zero(), U256::zero(), vec![], U256::zero()
        );
        assert!(!gkr_proof.bytes.is_empty(), "❌ GKR proof should not be empty");
        assert_eq!(solidity_proof.len(), 3, "❌ Solidity proof should have 3 components");
        assert_eq!(public_inputs.len(), 1, "❌ Expected single public input (GKR proof hash)");
        let is_valid = verify_proof(&gkr_proof, solidity_proof, public_inputs, &vk, &previous_proofs);
        assert!(is_valid, "❌ Hybrid liquidity proof verification failed!");
    }
}