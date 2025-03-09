use circuit::proof::{generate_proof, verify_proof};
use circuit::liquidity_circuit::{generate_liquidity_proof, verify_liquidity_proof};
use circuit::swap_circuit::{generate_swap_proof, verify_swap_proof};
use circuit::merkle_tree::MerkleTree;
use expander_transcript::Proof;
use circuit::libraries::types::{U256, U160};
use expander_compiler::circuit::config::BN254Config;
use std::time::Instant;

fn main() {
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];

    let decimals = U256::from(10).pow(U256::from(18));
    let swap_tests = vec![
        ("Swap Test 1", true, U256::from(5000) * decimals, U160::from_dec_str("7130534626283790383418955530240").unwrap(), U160::from_dec_str("7922816251426433759354395033600").unwrap(), U256::from(5_500_000), U256::from(3000)),
        ("Swap Test 2", false, U256::from(909) * decimals, U160::from_dec_str("8715097876569077135289834536960").unwrap(), U160::from_dec_str("7922816251426433759354395033600").unwrap(), U256::from(1_000_000), U256::zero()),
    ];

    // Test cases for LiquidityCircuit
    let liquidity_tests = vec![
        ("Liquidity Test 1", U256::from(0x1234), -100, 100, 1_000_000i128, 1, [0u8; 32], U256::from_dec_str("79228162514264337593543950336").unwrap()),
        ("Liquidity Test 2", U256::from(0x5678), -200, 200, -500_000i128, 10, [1u8; 32], U256::from_dec_str("8715097876569077135289834536960").unwrap()),
    ];

    // CrossChain test parameters
    let old_leaves = vec![U256::from(1), U256::from(2), U256::from(3), U256::from(4)];
    let old_tree = MerkleTree::new(old_leaves.clone());
    let old_state_root = old_tree.get_root();
    let mut new_leaves = old_leaves;
    new_leaves[0] = U256::from(5);
    let new_tree = MerkleTree::new(new_leaves);
    let new_state_root = new_tree.get_root();
    let leaf = U256::from(5);
    let merkle_proof = new_tree.get_proof(0);

    println!("===== GKR-Only Tests =====");

    // Test SwapCircuitGKR
    for (name, zero_for_one, amount, limit, current, liquidity, fee) in &swap_tests {
        println!("\nTesting: {}", name);
        let start = Instant::now();
        let proof = generate_swap_proof(*zero_for_one, *amount, *limit, *current, *liquidity, *fee);
        let gen_time = start.elapsed();
        println!("⏱ Generation Time: {:.2}µs", gen_time.as_micros() as f64);
        println!("Proof Size: {} bytes", proof.bytes.len());

        let verify_start = Instant::now();
        let valid = verify_swap_proof::<BN254Config>(&proof, *zero_for_one, *amount, *limit, *current, *liquidity, *fee, &previous_proofs);
        let verify_time = verify_start.elapsed();
        println!("Verification Result: {}", valid);
        println!("⏱ Verification Time: {:.2}µs", verify_time.as_micros() as f64);
        assert!(valid, "{} failed verification", name);
    }

    // Test LiquidityCircuit
    for (name, owner, tick_lower, tick_upper, delta, spacing, salt, sqrt_price) in &liquidity_tests {
        println!("\nTesting: {}", name);
        let start = Instant::now();
        let proof = generate_liquidity_proof(*owner, *tick_lower, *tick_upper, *delta, *spacing, *salt, *sqrt_price);
        let gen_time = start.elapsed();
        println!("⏱ Generation Time: {:.2}µs", gen_time.as_micros() as f64);
        println!("Proof Size: {} bytes", proof.bytes.len());

        let verify_start = Instant::now();
        let valid = verify_liquidity_proof::<BN254Config>(&proof, *owner, *tick_lower, *tick_upper, *delta, *spacing, *salt, *sqrt_price, &previous_proofs);
        let verify_time = verify_start.elapsed();
        println!("Verification Result: {}", valid);
        println!("⏱ Verification Time: {:.2}µs", verify_time.as_micros() as f64);
        assert!(valid, "{} failed verification", name);
    }

    println!("\n===== Hybrid GKR + Groth16 Tests =====");
    let type_titles = ["Type 0 - Swap", "Type 1 - Liquidity", "Type 2 - CrossChain"];

    // Test each circuit type
    for input_type in 0..3 {
        println!("\nTesting: {}", type_titles[input_type]);
        let (swap_params, liquidity_params, cross_chain_params) = match input_type {
            0 => (
                Some((swap_tests[0].1, swap_tests[0].2, swap_tests[0].3, swap_tests[0].4, swap_tests[0].5, swap_tests[0].6)),
                None,
                None,
            ),
            1 => (
                None,
                Some((liquidity_tests[0].1, liquidity_tests[0].2, liquidity_tests[0].3, liquidity_tests[0].4, liquidity_tests[0].5, liquidity_tests[0].6, liquidity_tests[0].7)),
                None,
            ),
            2 => (
                None,
                None,
                Some((old_state_root, new_state_root, leaf, merkle_proof.clone())),
            ),
            _ => unreachable!(),
        };

        let start = Instant::now();
        let cross_chain_params_clone = cross_chain_params.clone(); // Clone once
        let (gkr_proof, solidity_proof, public_inputs, verifying_key) = generate_proof(
            input_type.try_into().unwrap(),
            swap_params.map_or(false, |p| p.0),
            swap_params.map_or(U256::zero(), |p| p.1),
            swap_params.map_or(U256::zero(), |p| U256::from(p.2)),
            swap_params.map_or(U256::zero(), |p| U256::from(p.3)),
            swap_params.map_or(U256::zero(), |p| p.4),
            swap_params.map_or(U256::zero(), |p| p.5),
            liquidity_params.map_or(U256::zero(), |p| p.0),
            liquidity_params.map_or(0, |p| p.1),
            liquidity_params.map_or(0, |p| p.2),
            liquidity_params.map_or(0, |p| p.3),
            liquidity_params.map_or(0, |p| p.4),
            liquidity_params.map_or([0u8; 32], |p| p.5),
            liquidity_params.map_or(U256::zero(), |p| p.6),
            cross_chain_params_clone.clone().map_or(U256::zero(), |p| p.0),
            cross_chain_params_clone.clone().map_or(U256::zero(), |p| p.1),
            cross_chain_params_clone.clone().map_or(vec![], |p| p.3),
            cross_chain_params_clone.map_or(U256::zero(), |p| p.2),
            &previous_proofs,
        );
        let gen_time = start.elapsed();
        println!("⏱ Generation Time: {:.2}µs", gen_time.as_micros() as f64);
        println!("GKR Proof Size: {} bytes", gkr_proof.bytes.len());

        let verify_start = Instant::now();
        let valid = verify_proof(&gkr_proof, solidity_proof.clone(), public_inputs.clone(), &verifying_key, &previous_proofs);
        let verify_time = verify_start.elapsed();
        println!("Verification Result: {}", valid);
        println!("⏱ Verification Time: {:.2}µs", verify_time.as_micros() as f64);
        assert!(valid, "{} hybrid proof failed", type_titles[input_type]);
    }
}