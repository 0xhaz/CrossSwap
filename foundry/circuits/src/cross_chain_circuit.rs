use expander_compiler::frontend::{Define, Config, Variable, BasicAPI, API as RootBuilder};
use expander_compiler::circuit::ir::source::Irc;
use circuit_std_rs::poseidon_m31::*;
use expander_compiler::field::FieldArith;
use ethnum::U256;
use arith::{FieldForECC, FieldSerde, FieldSerdeError};

pub struct CrossChainCircuit {
    pub old_state_root: Variable,
    pub new_state_root: Variable,
    pub merkle_proof: Vec<Variable>,
}

/// ✅ Convert `Variable` into a **valid** `usize` within expected range
fn variable_to_usize<C: Config, B: BasicAPI<C>>(api: &mut B, v: &Variable, root_len: usize) -> usize
where
    C::CircuitField: FieldForECC,
{
    match api.constant_value(*v) {
        Some(value) => {
            let field_value = value.to_u256();
            let modulus = C::CircuitField::MODULUS;

            // ✅ Ensure the extracted value is within modulus
            let reduced_value = field_value % modulus;

            // ✅ Extract safely within range
            let extracted = (reduced_value & U256::new(0xFFFFFFFF)).as_u32() as usize;
            let safe_index = extracted % root_len;

            println!(
                "🔹 Clamping extracted value {} to valid range (0..{}): {}",
                extracted, root_len, safe_index
            );

            safe_index
        }
        None => panic!("❌ Failed to extract usize from Variable"),
    }
}

impl<C: Config> Define<C> for CrossChainCircuit {
    fn define(&self, builder: &mut RootBuilder<C>) {
        let old_root = self.old_state_root;
        let new_root = self.new_state_root;

        let (mut api, _input_vars, _public_vars) = RootBuilder::<C>::new(0, 0);

        let poseidon_params = PoseidonM31Params::new(
            &mut api,
            POSEIDON_M31X16_RATE,
            16,
            POSEIDON_M31X16_FULL_ROUNDS,
            POSEIDON_M31X16_PARTIAL_ROUNDS,
        );

        let proof_vars: Vec<Variable> = self.merkle_proof.iter().cloned().collect();
        let computed_root = poseidon_params.hash_to_state(&mut api, &proof_vars);

        println!("🔹 Old root: {:?}", old_root);
        println!("🔹 New root: {:?}", new_root);
        println!("🔹 Computed root values: {:?}", computed_root);

        if computed_root.is_empty() {
            panic!("❌ Computed root is empty! Poseidon hashing might be incorrect.");
        }

        let root_var = computed_root[0].clone();
        let root_index = variable_to_usize(&mut api, &root_var, computed_root.len());

        println!("🔹 Corrected Computed root[0] index: {}", root_index);
        
        if root_index >= computed_root.len() {
            println!(
                "❌ Computed root[0] index {} is out of range (len: {})",
                root_index,
                computed_root.len()
            );
            panic!("Index out of bounds!");
        }

        let corrected_root_var = computed_root[root_index].clone();
        let diff = builder.sub(new_root, corrected_root_var);
        builder.assert_is_zero(diff);
    }
}