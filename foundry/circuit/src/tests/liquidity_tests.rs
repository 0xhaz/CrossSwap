#[cfg(test)]
mod tests {
    use circuit_std_rs::poseidon_m31::*;
    use crate::liquidity_proof::{generate_liquidity_proof, verify_liquidity_proof};
    use expander_transcript::Proof;
    use expander_compiler::circuit::config::{BN254Config, Config};
    use std::collections::HashMap;
    use crate::liquidity_backend::LiquidityBackend;
    use expander_compiler::frontend::{API, BasicAPI};

    #[test]
    fn test_liquidity_proof_and_verification() {
        let mut backend = LiquidityBackend::new();

        let gkr_config = <BN254Config as Config>::DefaultGKRConfig::default();
        let field_config = <BN254Config as Config>::DefaultGKRFieldConfig::default();

        let chain_id = 1;
        let new_liquidity_state = <BN254Config as Config>::CircuitField::from(1000u32);

        let proof_1 = backend.update_liquidity(chain_id, new_liquidity_state, &gkr_config, &field_config)
            .expect("Failed to generate proof");

        assert!(backend.verify_liquidity(chain_id, &proof_1, &gkr_config, &field_config));

        let new_liquidity_state_2 = <BN254Config as Config>::CircuitField::from(2000u32);
        let proof_2 = backend.update_liquidity(chain_id, new_liquidity_state_2, &gkr_config, &field_config)
            .expect("Failed to generate proof");

        assert!(backend.verify_liquidity(chain_id, &proof_2, &gkr_config, &field_config));

        println!("✅ Liquidity state update and proofs verified successfully!");
    }

    #[test]
    fn test_poseidon_state_root_transition() {
        let previous_state_root = <BN254Config as Config>::CircuitField::from(1234u32);
        let new_liquidity_state = <BN254Config as Config>::CircuitField::from(5678u32);

        let mut poseidon_api = API::<BN254Config>::new(16, 1).0;
        let poseidon_params = PoseidonM31Params::new(&mut poseidon_api, 8, 16, 8, 14);

        let previous_var = poseidon_api.constant(previous_state_root);
        let new_var = poseidon_api.constant(new_liquidity_state);

        let computed_new_root_var = poseidon_params.hash_to_state(&mut poseidon_api, &[previous_var, new_var])[0];

        let computed_new_root = poseidon_api.constant_value(computed_new_root_var)
            .expect("Expected constant value but got a variable");

        assert_ne!(previous_state_root, computed_new_root);
        println!("✅ Poseidon state root transition verified successfully!");
    }
}