use expander_compiler::frontend::{API, Variable, Define, BasicAPI};
use circuit_std_rs::poseidon_m31::*;
use expander_compiler::circuit::config::{BN254Config, Config};

pub struct SharedLiquidityCircuit<T> {
    pub chain_id: u32,
    pub liquidity_state: T,
    pub previous_state_root: T,
    pub new_state_root: T,
}

impl<C: Config> Define<C> for SharedLiquidityCircuit<C::CircuitField> {
    fn define(&self, builder: &mut API<C>) {
        let chain_id = builder.constant(C::CircuitField::from(self.chain_id));
        let liquidity = builder.constant(self.liquidity_state);
        let previous_root = builder.constant(self.previous_state_root);
        let new_root = builder.constant(self.new_state_root);

        let mut poseidon_api = API::<C>::new(16, 1).0;
        let poseidon_params = PoseidonM31Params::new(&mut poseidon_api, 8, 16, 8, 14);

        let computed_new_root = poseidon_params.hash_to_state(&mut poseidon_api, &[previous_root, liquidity])[0];

        let diff = builder.sub(new_root, computed_new_root);
        builder.assert_is_zero(diff);
    }
}