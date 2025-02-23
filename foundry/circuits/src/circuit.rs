use expander_compiler::frontend::{Define, API};
use expander_compiler::frontend::BasicAPI;

pub struct SwapCircuit<T> {
    pub input_token: T,
    pub output_token: T,
    pub liquidity: T,
    pub slippage_tolerance: T,
    pub expected_output: T,
}

impl<C: expander_compiler::circuit::config::Config> Define<C> for SwapCircuit<C::CircuitField> {
    fn define(&self, builder: &mut API<C>) {
        let input_token = builder.constant(self.input_token);
        let _output_token = builder.constant(self.output_token);  // Prefixing with `_` to avoid warning
        let liquidity = builder.constant(self.liquidity);
        let slippage_tolerance = builder.constant(self.slippage_tolerance);
        let expected_output = builder.constant(self.expected_output);

        // Compute actual output after slippage
        let numerator = builder.mul(input_token, liquidity);
        let denominator = builder.add(liquidity, input_token);
        let actual_output = builder.div(numerator, denominator, false);

        // Ensure actual output meets slippage tolerance
        let slippage_diff = builder.sub(actual_output, expected_output);
        let max_slippage = builder.mul(expected_output, slippage_tolerance);
        builder.assert_is_equal(slippage_diff, max_slippage);  
    }
}