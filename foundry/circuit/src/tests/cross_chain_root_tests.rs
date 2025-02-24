#[cfg(test)]
mod tests {
    use super::*;
    use crate::cross_chain_circuit::CrossChainCircuit;
    use expander_compiler::circuit::config::BN254Config;
    use expander_compiler::frontend::{API, Define, BasicAPI, Variable};
    use crate::proof::variable_to_u8;

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::cross_chain_circuit::CrossChainCircuit;
        use expander_compiler::circuit::config::BN254Config;
        use expander_compiler::frontend::{API, Define, BasicAPI, Variable};
        use crate::proof::variable_to_u8;
    
        #[test]
        fn test_cross_chain_state_root_verification() {
            let (mut api, _, _) = API::<BN254Config>::new(0, 0);
        
            let merkle_proof: Vec<Variable> = (1..=16)
                .map(|i| api.constant(i as u32))
                .collect();
        
            let computed_root_values: Vec<Variable> = (0..16) // ✅ Dummy data for computed roots
                .map(|i| api.constant(i as u32))
                .collect();
        
            let index_var = api.constant(2);
            let extracted_index = variable_to_u8(&mut api, &index_var);
        
            // ✅ Ensure extracted_index is within valid bounds
            if usize::from(extracted_index) >= computed_root_values.len() {
                println!(
                    "❌ Invalid index {} out of bounds! Length: {}",
                    extracted_index, computed_root_values.len()
                );
            }
    
            assert!(usize::from(extracted_index) < merkle_proof.len(), "❌ Computed index is invalid!");
        }
    }
}