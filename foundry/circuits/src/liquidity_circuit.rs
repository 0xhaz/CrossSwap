use expander_compiler::frontend::{
    Define, Config, Variable, BasicAPI, API as RootBuilder,
};
use expander_compiler::circuit::config::BN254Config;
use expander_compiler::circuit::ir::source::Irc;
use circuit_std_rs::poseidon_m31::*;
use expander_compiler::frontend::internal::DumpLoadVariables; 
use expander_transcript::Proof;
use rand::{thread_rng, Rng};
use expander_compiler::field::FieldArith;


pub struct LiquidityCircuit {
    pub user_balance: Variable,
    pub liquidity_added: Variable,
    pub pool_total_liquidity: Variable,
    pub expected_new_total: Variable,
}

impl<C: Config> Define<C> for LiquidityCircuit {
    fn define(&self, builder: &mut RootBuilder<C>) {
        let sum = builder.add(self.user_balance, self.liquidity_added); 
        builder.assert_is_equal(sum, self.expected_new_total); 
    }
}

fn variable_to_u32<C: Config>(api: &mut RootBuilder<C>, v: &Variable) -> u32 {
    match api.constant_value(*v) {
        Some(value) => value.as_u32_unchecked(), 
        None => panic!("Failed to extract u32 from Variable"),
    }
}

pub fn generate_liquidity_proof(
    user_balance: u32,
    liquidity_added: u32,
    pool_total_liquidity: u32,
    expected_new_total: u32,
) -> Proof {
    let mut rng = thread_rng();

    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(0, 0); // ✅ FIX: Use `new()`

    let circuit = LiquidityCircuit {
        user_balance: api.constant(user_balance),
        liquidity_added: api.constant(liquidity_added),
        pool_total_liquidity: api.constant(pool_total_liquidity),
        expected_new_total: api.constant(expected_new_total),
    };

    circuit.define(&mut api);

    let proof_data: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    Proof { bytes: proof_data }
}

pub fn verify_liquidity_proof(proof: &Proof) -> bool {
    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(0, 0); // ✅ FIX

    let poseidon_params = PoseidonM31Params::new(
        &mut api,
        POSEIDON_M31X16_RATE,
        16,
        POSEIDON_M31X16_FULL_ROUNDS,
        POSEIDON_M31X16_PARTIAL_ROUNDS
    );

    let proof_vars: Vec<Variable> = proof.bytes.iter()
        .map(|&b| api.constant(b as u32)) 
        .collect();

    let hash_result = poseidon_params.hash_to_state(&mut api, &proof_vars);

    proof.bytes.ends_with(
        &hash_result.iter()
            .map(|v| variable_to_u32::<BN254Config>(&mut api, v).to_le_bytes()[0]) 
            .collect::<Vec<u8>>(),
    )
}