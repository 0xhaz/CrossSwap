use circuit::circuit::SwapCircuit; // ✅ Fix: Import `SwapCircuit`
use expander_compiler::frontend::{API, Define};  
use expander_compiler::circuit::config::{BN254Config, Config}; 
use expander_compiler::field::BN254; // ✅ Import BN254 (Field Type)
use expander_transcript::Proof;
use circuit::proof::{generate_gkr_proof, verify_gkr_proof};

fn main() {
    println!("Initializing circuit...");

    // ✅ Fix: Use SwapCircuit<BN254> instead of BN254Config
    let circuit = SwapCircuit::<BN254> {
        input_token: BN254::from(3u32),   
        output_token: BN254::from(4u32),  
        liquidity: BN254::from(1000u32),
        slippage_tolerance: BN254::from(5u32),
        expected_output: BN254::from(12u32),
    };

    let (mut root_builder, _input_variables, _public_input_variables) = API::<BN254Config>::new(3, 1);
    
    circuit.define(&mut root_builder);

    // ✅ Fix: Remove generic arguments
    let proof_1 = generate_gkr_proof(&[]);  
    println!("Generated Proof 1: {:?}", proof_1);

    // ✅ Fix: Remove generic arguments
    let proof_2 = generate_gkr_proof(&[proof_1.clone()]);  
    println!("Generated Proof 2 (compressed): {:?}", proof_2);

    // ✅ Fix: Remove generic arguments
    let is_valid_1 = verify_gkr_proof(&proof_1, &[]);  
    println!("Proof 1 verification result: {}", is_valid_1);

    // ✅ Fix: Remove generic arguments
    let is_valid_2 = verify_gkr_proof(&proof_2, &[proof_1]);  
    println!("Proof 2 verification result: {}", is_valid_2);
}