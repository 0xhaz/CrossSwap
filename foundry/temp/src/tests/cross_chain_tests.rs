#[cfg(test)]
mod tests {
    use crate::cross_chain_circuit::CrossChainCircuit;
    use expander_compiler::circuit::config::BN254Config;
    use expander_compiler::frontend::{API, Define, BasicAPI, Variable};
    use crate::proof::{generate_gkr_proof, verify_gkr_proof, u256_to_bn254, generate_proof, verify_proof};
    use expander_transcript::Proof;
    use primitive_types::U256;

    #[test]
    fn test_cross_chain_state_root_verification() {
        let (mut api, _, _) = API::<BN254Config>::new(0, 0);
    
        let leaf = u256_to_bn254(U256::from(5));
        let merkle_proof: Vec<Variable> = vec![
            api.constant(u256_to_bn254(U256::from(2))),
            api.constant(u256_to_bn254(U256::from_dec_str("21782172027182199195932805524207718591141883593739784509059183843586302181518").unwrap())),
            api.constant(u256_to_bn254(U256::from(4))),
        ];
        let old_state_root = api.constant(u256_to_bn254(U256::from_dec_str("11169319127036400609278385814774371743379610577468079623686554067272392515506").unwrap()));
        let new_state_root = api.constant(u256_to_bn254(U256::from_dec_str("11196751019539528149259467034886616587906299194942945944013612589222474269").unwrap())); // Match computed root
    
        let circuit = CrossChainCircuit {
            old_state_root,
            new_state_root,
            merkle_proof,
            leaf: api.constant(leaf),
        };
        circuit.define(&mut api);
    
        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let gkr_proof = generate_gkr_proof(&previous_proofs);
        assert!(!gkr_proof.bytes.is_empty(), "❌ GKR proof for cross-chain should not be empty");
        assert!(verify_gkr_proof(&gkr_proof, &previous_proofs), "❌ GKR proof verification failed");
    }

    #[test]
    fn test_hybrid_cross_chain_proof() {
        let leaf = U256::from(5);
        let merkle_proof = vec![
            U256::from(2),
            U256::from_dec_str("21782172027182199195932805524207718591141883593739784509059183843586302181518").unwrap(),
            U256::from(4),
        ];
        let old_state_root = U256::from_dec_str("11169319127036400609278385814774371743379610577468079623686554067272392515506").unwrap();
        let new_state_root = U256::from_dec_str("11196751019539528149259467034886616587906299194942945944013612589222474269").unwrap(); // Match computed root

        let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
        let (gkr_proof, solidity_proof, public_inputs, vk) = generate_proof(
            2, U256::zero(), U256::zero(), U256::zero(), U256::zero(), U256::zero(), U256::zero(), U256::zero(),
            &previous_proofs, old_state_root, new_state_root, merkle_proof, leaf
        );
        assert!(!gkr_proof.bytes.is_empty(), "❌ GKR proof should not be empty");
        assert_eq!(solidity_proof.len(), 3, "❌ Solidity proof should have 3 components");
        assert_eq!(public_inputs.len(), 1, "❌ Expected single public input (new_state_root hash)");
        assert_eq!(public_inputs[0], new_state_root, "❌ Public input should match new_state_root");
        let is_valid = verify_proof(&gkr_proof, solidity_proof, public_inputs, &vk, &previous_proofs);
        assert!(is_valid, "❌ Hybrid cross-chain proof verification failed!");
    }
}