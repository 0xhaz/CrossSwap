#[cfg(test)]
mod tests {
    use crate::swap_circuit::{generate_swap_proof, verify_swap_proof};
    use expander_compiler::circuit::config::BN254Config;
    use expander_transcript::Proof;
    use primitive_types::U256;

    fn scale(value: u64) -> U256 {
        U256::from(value) * U256::from(10).pow(U256::from(18))
    }

    #[test]
    fn test_valid_swap() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_swap_proof(
            scale(10),          // input_token
            scale(9),           // output_token
            scale(1000),        // liquidity
            scale(5) / 100,     // 5% slippage tolerance
            scale(990) / 100,   // 9.9 * 10^18
        );
        let is_valid = verify_swap_proof::<BN254Config>(
            &proof,
            scale(10),
            scale(9),
            scale(1000),
            scale(5) / 100,
            scale(990) / 100,
            &previous_proofs,
        );
        assert!(is_valid, "Valid swap proof should pass verification");
    }

    #[test]
    fn test_invalid_slippage() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_swap_proof(
            scale(100),      // input_token
            scale(50),       // output_token
            scale(1000),    // liquidity
            scale(1) / 100, // 1% slippage tolerance
            scale(40),       // expected_output
        );
        let is_valid = verify_swap_proof::<BN254Config>(
            &proof,
            scale(100),
            scale(50),
            scale(1000),
            scale(1) / 100,
            scale(40),
            &previous_proofs,
        );
        assert!(!is_valid, "Invalid slippage should fail verification");
    }

    #[test]
    fn test_zero_inputs() {
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let proof = generate_swap_proof(
            scale(0),       // input_token
            scale(0),       // output_token
            scale(0),       // liquidity
            scale(1) / 100, // 1% slippage tolerance
            scale(0),       // expected_output
        );
        let is_valid = verify_swap_proof::<BN254Config>(
            &proof,
            scale(0),
            scale(0),
            scale(0),
            scale(1) / 100,
            scale(0),
            &previous_proofs,
        );
        assert!(is_valid, "Zero inputs should pass verification");
    }
}