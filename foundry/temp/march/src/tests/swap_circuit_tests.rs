#[cfg(test)]
mod tests {
    use crate::swap_circuit::{SwapCircuitGKR};
    use crate::proof::{generate_gkr_proof, verify_gkr_proof};
    use crate::libraries::types::{U256, U160, I256};
    use expander_transcript::Proof;

    fn scale(value: u64) -> I256 {
        I256::from(u128::from(value)) * I256::from(U256::from(10).pow(U256::from(18)))
    }

    #[test]
    fn test_valid_swap() {
        let circuit = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(10), // 10 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(7130534626283790383418955530240u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 1000,
            fee_pips: 3000, // 0.3%
            hook_data: vec![], // No hook
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Valid swap proof should pass verification");
    }

    #[test]
    fn test_swap_with_before_hook() {
        let circuit = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(20), // 20 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(7130534626283790383418955530240u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 2000,
            fee_pips: 3000, // 0.3%
            hook_data: vec![1], // Before hook: extra 1% fee
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Swap proof with before hook failed verification");
    }

    #[test]
    fn test_swap_with_after_hook() {
        let circuit = SwapCircuitGKR {
            zero_for_one: false,
            amount_specified: scale(15), // 15 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(8715097876569077135289834536960u128),
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 1500,
            fee_pips: 1000, // 0.1%
            hook_data: vec![2], // After hook: 1% reduction in amount0
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Swap proof with after hook failed verification");
    }

    #[test]
    fn test_invalid_slippage() {
        let circuit = SwapCircuitGKR {
            zero_for_one: true,
            amount_specified: scale(100), // 100 tokens in 10^18 units
            sqrt_price_limit_x96: U160::from(79228162514264337593543950336u128), // Tight limit
            sqrt_price_current_x96: U160::from(7922816251426433759354395033600u128),
            liquidity: 1000,
            fee_pips: 3000, // 0.3%
            hook_data: vec![], // No hook
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Swap proof should verify regardless of slippage (GKR only checks proof consistency)");
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
            hook_data: vec![], // No hook
        };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(is_valid, "Zero inputs should pass verification");
    }

    #[test]
    fn test_swap_proof_invalid_hash() {
        let proof = Proof { bytes: vec![0xFF; 32] };
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let is_valid = verify_gkr_proof(&proof, &previous_proofs);
        assert!(!is_valid, "Invalid swap proof should fail verification");
    }
}