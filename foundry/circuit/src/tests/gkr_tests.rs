#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::{generate_gkr_proof, verify_gkr_proof};
    use expander_transcript::Proof;

    #[test]
    fn test_gkr_proof_generation() {
        let proofs = vec![];
        let proof = generate_gkr_proof(&proofs);
        // println!("🔹 Generated Proof Bytes: {:?}", proof.bytes);
        assert!(!proof.bytes.is_empty(), "❌ Generated proof should not be empty");
    }

    #[test]
    fn test_gkr_proof_verification() {
        let proofs = vec![];
        let proof = generate_gkr_proof(&proofs);

        // println!("🔹 Proof bytes: {:?}", proof.bytes);

        let is_valid = verify_gkr_proof(&proof, &proofs);
        assert!(is_valid, "❌ GKR Proof verification failed!");
    }
}