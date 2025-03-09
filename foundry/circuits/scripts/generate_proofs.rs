use circuit::liquidity_circuit::{generate_liquidity_proof, verify_liquidity_proof};
use circuit::swap_circuit::{generate_swap_proof, verify_swap_proof};
use expander_compiler::circuit::config::BN254Config;
use expander_transcript::Proof;
use primitive_types::U256;
use serde_json;
use std::fs::File;
use std::io::Write;

fn scale(value: u64) -> U256 {
    U256::from(value) * U256::from(10).pow(U256::from(18))
}

fn main() {
    // // Previous proofs for consistency
    // let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

    // // Liquidity Circuit Inputs
    // let liquidity_inputs = vec![
    //     // Valid case: sufficient balance
    //     (scale(100), scale(50), scale(1000), scale(1050)),
    //     // Edge case: insufficient balance
    //     (scale(10), scale(20), scale(1000), scale(1020)),
    //     // Zero case: zero balance
    //     (scale(0), scale(0), scale(0), scale(0)),
    // ];

    // // Swap Circuit Inputs (aligned with tests)
    // let swap_inputs = vec![
    //     // Valid swap: 5% tolerance
    //     (scale(10), scale(9), scale(1000), scale(5) / 100, scale(990) / 100), // 9.9 * 10^18
    //     // Invalid swap: exceeds slippage
    //     (scale(100), scale(50), scale(1000), scale(5) / 100, scale(40)),
    //     // Zero swap: zero inputs
    //     (scale(0), scale(0), scale(0), scale(5) / 100, scale(0)),
    // ];

    // let mut outputs = Vec::new();

    // // Generate proofs for Liquidity Circuit
    // for(user_balance, liquidity_added, pool_total_liquidity, expected_new_total) in liquidity_inputs {
    //     let proof = generate_liquidity_proof(
    //         user_balance,
    //         liquidity_added,
    //         pool_total_liquidity,
    //         expected_new_total,
    //     );
    //     let is_valid = verify_liquidity_proof::<BN254Config>(
    //         &proof,
    //         user_balance,
    //         liquidity_added,
    //         pool_total_liquidity,
    //         expected_new_total,
    //         &previous_proofs,
    //     );

    //     let proof_hex = hex::encode(&proof.bytes);
    //     outputs.push(serde_json::json!({
    //         "circuit": "LiquidityCircuit",
    //         "inputs": {
    //             "user_balance": user_balance.to_string(),
    //             "liquidity_added": liquidity_added.to_string(),
    //             "pool_total_liquidity": pool_total_liquidity.to_string(),
    //             "expected_new_total": expected_new_total.to_string(),
    //         },
    //         "proof": proof_hex,
    //         "verified": is_valid,
    //     }));
    // }

    // // Generate proofs for Swap Circuit
    // for(input_token, output_token, liquidity, slippage_tolerance, expected_output) in swap_inputs {
    //     let proof = generate_swap_proof(
    //         input_token,
    //         output_token,
    //         liquidity,
    //         slippage_tolerance,
    //         expected_output,
    //     );
    //     let is_valid = verify_swap_proof::<BN254Config>(
    //         &proof,
    //         input_token,
    //         output_token,
    //         liquidity,
    //         slippage_tolerance,
    //         expected_output,
    //         &previous_proofs,
    //     );

    //     let proof_hex = hex::encode(&proof.bytes);
    //     outputs.push(serde_json::json!({
    //         "circuit": "SwapCircuitGKR",
    //         "inputs": {
    //             "input_token": input_token.to_string(),
    //             "output_token": output_token.to_string(),
    //             "liquidity": liquidity.to_string(),
    //             "slippage_tolerance": slippage_tolerance.to_string(),
    //             "expected_output": expected_output.to_string(),
    //         },
    //         "proof": proof_hex,
    //         "verified": is_valid,
    //     }));
    // }

    // // Write proofs to file
    // let json_output = serde_json::to_string_pretty(&outputs).unwrap();
    // let mut file = File::create("scripts/generated_proofs.json").unwrap();
    // file.write_all(json_output.as_bytes()).unwrap();
    // println!("Proofs generated and saved to 'scripts/generated_proofs.json'");
}