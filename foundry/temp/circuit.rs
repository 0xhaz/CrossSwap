use expander_compiler::frontend::{Define, API};
use expander_compiler::circuit::config::BN254Config;
use expander_compiler::frontend::BasicAPI;

pub struct SimpleCircuit<T> {
    pub x: T,
    pub y: T,
    pub result: T,
    pub is_addition: bool,
}

impl<C: expander_compiler::circuit::config::Config> Define<C> for SimpleCircuit<C::CircuitField> {
    fn define(&self, builder: &mut API<C>) {
        let x = builder.constant(self.x);
        let y = builder.constant(self.y);
        let expected_result = builder.constant(self.result);
        
        let computed_result = if self.is_addition {
            builder.add(x, y)
        } else {
            builder.mul(x, y)
        };
        let diff = builder.sub(expected_result, computed_result);
        builder.assert_is_zero(diff);
    }
}