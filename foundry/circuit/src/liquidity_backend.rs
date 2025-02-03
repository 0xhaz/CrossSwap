use expander_compiler::frontend::{API, Variable, BasicAPI};
use circuit_std_rs::poseidon_m31::*;
use crate::liquidity_proof::{generate_liquidity_proof, verify_liquidity_proof};
use expander_transcript::Proof;
use expander_compiler::circuit::config::{BN254Config, Config};
use std::collections::HashMap;

pub struct LiquidityBackend {
    // (previous_state_root, current_state_root)
    liquidity_states: HashMap<u32, (<BN254Config as Config>::CircuitField, <BN254Config as Config>::CircuitField)>,
    // (previous_proof, current_proof)
    proofs: HashMap<u32, (Option<Proof>, Proof)>,
}

impl LiquidityBackend {
    pub fn new() -> Self {
        Self {
            liquidity_states: HashMap::new(),
            proofs: HashMap::new(),
        }
    }

    pub fn update_liquidity(
        &mut self,
        chain_id: u32,
        new_liquidity_state: <BN254Config as Config>::CircuitField,
        gkr_config: &<BN254Config as Config>::DefaultGKRConfig,
        field_config: &<BN254Config as Config>::DefaultGKRFieldConfig,
    ) -> Option<Proof> {
           // Get previous state root (current becomes previous after update)
        let (previous_state_root, _) = self.liquidity_states
        .get(&chain_id)
        .copied()
        .unwrap_or_else(|| (
            <BN254Config as Config>::CircuitField::from(0u32),
            <BN254Config as Config>::CircuitField::from(0u32)
        ));

        let mut poseidon_api = API::<BN254Config>::new(16, 1).0;
        let poseidon_params = PoseidonM31Params::new(&mut poseidon_api, 8, 16, 8, 14);
        let previous_var = poseidon_api.constant(previous_state_root);
        let new_var = poseidon_api.constant(new_liquidity_state);
        let new_state_root_var = poseidon_params.hash_to_state(
            &mut poseidon_api, 
            &[previous_var, new_var]
        )[0];
       // Now perform the hash operation
        let new_state_root_var = poseidon_params.hash_to_state(
            &mut poseidon_api, 
            &[previous_var, new_var]
        )[0];
        
        let new_state_root = poseidon_api.constant_value(new_state_root_var)
            .expect("Expected constant value but got a variable");

        // Retrieve previous proof
        let previous_proof = self.proofs.get(&chain_id)
            .map(|(p, _)| p.clone())
            .unwrap_or(None);

        // Generate proof
        let proof = generate_liquidity_proof::<BN254Config>(
            gkr_config,
            field_config,
            previous_proof.as_ref(),
            previous_state_root,
            new_state_root,
        );

        // Update state and proofs
        self.liquidity_states.insert(chain_id, (previous_state_root, new_state_root));
        self.proofs.insert(chain_id, (previous_proof.clone(), proof.clone()));

        Some(proof)
    }

    pub fn verify_liquidity(
        &self,
        chain_id: u32,
        proof: &Proof,
        gkr_config: &<BN254Config as Config>::DefaultGKRConfig,
        field_config: &<BN254Config as Config>::DefaultGKRFieldConfig,
    ) -> bool {
        let (prev_state, expected_state) = match self.liquidity_states.get(&chain_id) {
            Some((p, c)) => (*p, *c),
            None => {
                println!("🚨 No state found for chain {}", chain_id);
                return false;
            }
        };

        let (previous_proof, _) = self.proofs.get(&chain_id)
            .map(|(p, _)| (p.as_ref(), p))
            .unwrap_or((None, &None));

        verify_liquidity_proof::<BN254Config>(
            gkr_config,
            field_config,
            proof,
            previous_proof,
            prev_state,
            expected_state,
        )
    }
}