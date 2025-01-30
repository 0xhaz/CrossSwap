use circuit::circuit::SimpleCircuit;
use expander_compiler::frontend::{API, Define};  
use expander_compiler::circuit::config::{BN254Config, Config}; 
use expander_transcript::Proof;
use circuit::proof::{generate_gkr_proof, verify_gkr_proof};

fn main() {
    println!("Initializing circuit...");

    let circuit = SimpleCircuit {
        x: <BN254Config as Config>::CircuitField::from(3u32),   
        y: <BN254Config as Config>::CircuitField::from(4u32),   
        result: <BN254Config as Config>::CircuitField::from(12u32), 
        is_addition: false,
    };

    let (mut root_builder, _input_variables, _public_input_variables) = API::<BN254Config>::new(3, 1);
    
    circuit.define(&mut root_builder);

    // ✅ Instantiate Config Objects Instead of Accessing as Constants
    let gkr_config = <BN254Config as Config>::DefaultGKRConfig::default();
    let field_config = <BN254Config as Config>::DefaultGKRFieldConfig::default();

    // ✅ Generate first proof
    let proof_1 = generate_gkr_proof::<BN254Config>(&gkr_config, &field_config, None);
    println!("Generated Proof 1: {:?}", proof_1);

    // ✅ Generate recursive proof (compressed)
    let proof_2 = generate_gkr_proof::<BN254Config>(&gkr_config, &field_config, Some(&proof_1));
    println!("Generated Proof 2 (compressed): {:?}", proof_2);

    // ✅ Verify first proof
    let is_valid_1 = verify_gkr_proof::<BN254Config>(&gkr_config, &field_config, &proof_1, None);
    println!("Proof 1 verification result: {}", is_valid_1);

    // ✅ Verify compressed proof (recursive check)
    let is_valid_2 = verify_gkr_proof::<BN254Config>(&gkr_config, &field_config, &proof_2, Some(&proof_1));
    println!("Proof 2 verification result: {}", is_valid_2);
}