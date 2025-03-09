use circuit::proof::{generate_proof, verify_proof};
use circuit::liquidity_circuit::{generate_liquidity_proof, verify_liquidity_proof};
use circuit::swap_circuit::{SwapCircuitGKR, generate_swap_proof, verify_swap_proof};
use expander_transcript::Proof;
use primitive_types::U256;
use expander_compiler::circuit::config::BN254Config;
use std::time::Instant;

fn main() {
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

    let user_balance = U256::from(1000);
    let liquidity_added = U256::from(500);
    let pool_total_liquidity = U256::from(2000);
    let expected_new_total = U256::from(2500);

    // Scaling factor: 1e18
    let scale = U256::from(10).pow(U256::from(18));

    // Test inputs (human-readable, scaled to 10^18)
    let input_token = U256::from(10) * scale; // 10 tokens
    let output_token = U256::from(9) * scale; // 9 tokens (output)
    let liquidity = U256::from(1000) * scale; // 1000 tokens liquidity
    let slippage_tolerance = U256::from(1) * scale / U256::from(100); // 1% slippage = 0.01 * scale
    let expected_output = U256::from(9) * scale; // 9 tokens (expected)

    // Test GKR-only proof for SwapCircuit
    println!("=== Testing GKR-Only Swap Proof ===");
    let proof = generate_swap_proof(
        input_token,
        output_token,
        liquidity,
        slippage_tolerance,
        expected_output,
    );
    println!("Proof generated: {:?}", hex::encode(&proof.bytes));

    // Verify proof
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
    let is_valid = verify_swap_proof::<BN254Config>(
        &proof,
        input_token,
        output_token,
        liquidity,
        slippage_tolerance,
        expected_output,
        &previous_proofs,
    );
    println!("Verification result: {}", is_valid);

    // Quick check for expected output
    if is_valid {
        println!("Swap proof verified successfully!");
    } else {
        println!("Swap proof verification failed!");
    }

    // Test GKR-only proof for LiquidityCircuit
    println!("=== Testing GKR-Only Liquidity Proof ===");
    let gkr_start = Instant::now();
    let gkr_proof = generate_liquidity_proof(
        user_balance,
        liquidity_added,
        pool_total_liquidity,
        expected_new_total,
    );
    let gkr_gen_time = gkr_start.elapsed();
    println!("GKR-only Proof: {:?}", gkr_proof);
    println!("GKR-only Proof Size: {} bytes", gkr_proof.bytes.len());
    println!("GKR-only Generation Time: {:?}", gkr_gen_time);

    let gkr_verify_start = Instant::now();
    let gkr_valid = verify_liquidity_proof::<BN254Config>(
        &gkr_proof,
        user_balance,
        liquidity_added,
        pool_total_liquidity,
        expected_new_total,
        &previous_proofs,
    );
    let gkr_verify_time = gkr_verify_start.elapsed();
    println!("GKR-only Proof Valid: {}", gkr_valid);
    println!("GKR-only Verification Time: {:?}", gkr_verify_time);

    // Test Hybrid GKR + Groth16 proof with real cross-chain data from MerkleRootContract
    println!("\n=== Testing Hybrid GKR + Groth16 Liquidity Proof ===");
    let input_type = 2; // Cross-chain

    // Real data from your MerkleRootContract (replace with actual queried values)
    let old_state_root = U256::from_little_endian(
        &hex::decode("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap()
    );
    let new_state_root = U256::from_little_endian(
        &hex::decode("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890").unwrap()
    );
    let merkle_proof = vec![
        U256::from_little_endian(&hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap()),
        U256::from_little_endian(&hex::decode("2222222222222222222222222222222222222222222222222222222222222222").unwrap()),
        // Add more proof elements based on your tree depth
    ];

    let hybrid_start = Instant::now();
    let (hybrid_gkr_proof, solidity_proof, public_inputs, verifying_key) = generate_proof(
        input_type,
        U256::from(0),
        U256::from(0),
        liquidity_added,
        U256::from(0),
        expected_new_total,
        user_balance,
        pool_total_liquidity,
        &previous_proofs,
        old_state_root,
        new_state_root,
        merkle_proof,
    );
    let hybrid_gen_time = hybrid_start.elapsed();
    println!("Hybrid GKR Proof: {:?}", hybrid_gkr_proof);
    println!("Solidity Proof (Groth16): {:?}", solidity_proof);
    println!("Public Inputs (Groth16): {:?}", public_inputs);
    println!(
        "Circuit Inputs: [{}, {}, {}, {}, {}]",
        input_type, pool_total_liquidity, liquidity_added, U256::zero(), expected_new_total
    );
    println!("Verifying Key: {:?}", verifying_key);
    println!("Hybrid GKR Proof Size: {} bytes", hybrid_gkr_proof.bytes.len());
    println!("Hybrid Generation Time: {:?}", hybrid_gen_time);

    let hybrid_verify_start = Instant::now();
    let hybrid_valid = verify_proof(
        &hybrid_gkr_proof,
        solidity_proof.clone(),
        public_inputs.clone(),
        &verifying_key,
        &previous_proofs,
    );
    let hybrid_verify_time = hybrid_verify_start.elapsed();
    println!("Hybrid GKR + Groth16 Proof Valid: {}", hybrid_valid);
    println!("Hybrid Verification Time: {:?}", hybrid_verify_time);
}