use circuit::proof::generate_gkr_proof;
use circuit::proof::verify_gkr_proof;
use circuit::liquidity_circuit::LiquidityCircuit;
use circuit::swap_circuit::{SwapCircuitGKR, generate_swap_proof, BalanceDelta};
use circuit::cross_chain_circuit::CrossChainCircuit;
use circuit::merkle_tree::MerkleTree;
use expander_transcript::Proof;
use circuit::libraries::types::{U256, U160, I256};
use expander_compiler::circuit::config::BN254Config;
use std::time::Instant;

fn main() {
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
    let decimals = U256::from(10).pow(U256::from(18));
    let swap_tests = vec![
        (
            "Swap Test 1",
            true,
            I256::from(5000i128) * I256::from(decimals), // Exact out, token1
            U160::from_dec_str("7130534626283790383418955530240").unwrap(),
            U160::from_dec_str("7922816251426433759354395033600").unwrap(),
            5_500_000u128,
            3000u32,
            vec![], // No hook
        ),
        (
            "Swap Test 2",
            false,
            I256::from(909i128) * I256::from(decimals), // Exact out, token0
            U160::from_dec_str("8715097876569077135289834536960").unwrap(),
            U160::from_dec_str("7922816251426433759354395033600").unwrap(),
            1_000_000u128,
            0u32,
            vec![], // No hook
        ),
    ];
    let liquidity_tests = vec![
        (
            "Liquidity Test 1",
            U256::from(0x1234),
            -100,
            100,
            1_000_000i128,
            1,
            [0u8; 32],
            U256::from_dec_str("79228162514264337593543950336").unwrap(),
            vec![], // No hook
        ),
        (
            "Liquidity Test 2",
            U256::from(0x5678),
            -200,
            200,
            -500_000i128,
            10,
            [1u8; 32],
            U256::from_dec_str("8715097876569077135289834536960").unwrap(),
            vec![], // No hook
        ),
    ];
    let old_leaves = vec![U256::from(1), U256::from(2), U256::from(3), U256::from(4)];
    let old_tree = MerkleTree::new(old_leaves.clone());
    let old_state_root = old_tree.get_root();
    let mut new_leaves = old_leaves;
    new_leaves[0] = U256::from(5);
    let new_tree = MerkleTree::new(new_leaves);
    let new_state_root = new_tree.get_root();
    let leaf = U256::from(5);
    let merkle_proof = new_tree.get_proof(0);
    let leaf_index = 0;

    println!("===== GKR-Only Tests =====");

    for (name, zero_for_one, amount, limit, current, liquidity, fee, hook_data) in &swap_tests {
        println!("\nTesting: {}", name);
        println!("=============================================");
        println!("Inputs:");
        println!("  zero_for_one: {}", zero_for_one);
        println!("  amount_specified: {}", amount);
        println!("  sqrt_price_limit_x96: {}", limit);
        println!("  sqrt_price_current_x96: {}", current);
        println!("  liquidity: {}", liquidity);
        println!("  fee_pips: {}", fee);
        println!("  hook_data: {:?}", hook_data);
        println!("=============================================");
        let circuit = SwapCircuitGKR {
            zero_for_one: *zero_for_one,
            amount_specified: amount.clone(),
            sqrt_price_limit_x96: *limit,
            sqrt_price_current_x96: *current,
            liquidity: *liquidity,
            fee_pips: *fee,
            hook_data: hook_data.clone(),
        };
        let start = Instant::now();
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let gen_time = start.elapsed();
        println!("⏱ Generation Time: {:.2}µs", gen_time.as_micros() as f64);
        println!("Proof Size: {} bytes", proof.bytes.len());
        let verify_start = Instant::now();
        let valid = verify_gkr_proof(&proof, &previous_proofs);
        let verify_time = verify_start.elapsed();
        println!("Verification Result: {}", valid);
        println!("⏱ Verification Time: {:.2}µs", verify_time.as_micros() as f64);
        assert!(valid, "{} failed verification", name);
        println!("=============================================");
    }

    for (name, owner, tick_lower, tick_upper, delta, spacing, _salt, sqrt_price, hook_data) in &liquidity_tests {
        println!("\nTesting: {}", name);
        println!("=============================================");
        println!("Inputs:");
        println!("  owner: {}", owner);
        println!("  tick_lower: {}", tick_lower);
        println!("  tick_upper: {}", tick_upper);
        println!("  liquidity_delta: {}", delta);
        println!("  tick_spacing: {}", spacing);
        println!("  sqrt_price_current_x96: {}", sqrt_price);
        println!("  hook_data: {:?}", hook_data);
        println!("=============================================");
        let circuit = LiquidityCircuit {
            owner: *owner,
            tick_lower: *tick_lower,
            tick_upper: *tick_upper,
            liquidity_delta: *delta,
            tick_spacing: *spacing,
            salt: *_salt,
            sqrt_price_current_x96: *sqrt_price,
            hook_data: hook_data.clone(),
        };
        let start = Instant::now();
        let proof = generate_gkr_proof(&circuit, &previous_proofs);
        let gen_time = start.elapsed();
        println!("⏱ Generation Time: {:.2}µs", gen_time.as_micros() as f64);
        println!("Proof Size: {} bytes", proof.bytes.len());
        let verify_start = Instant::now();
        let valid = verify_gkr_proof(&proof, &previous_proofs);
        let verify_time = verify_start.elapsed();
        println!("Verification Result: {}", valid);
        println!("⏱ Verification Time: {:.2}µs", verify_time.as_micros() as f64);
        assert!(valid, "{} failed verification", name);
        println!("=============================================");
    }

    println!("\nTesting: CrossChain Test");
    println!("=============================================");
    println!("Inputs:");
    println!("  old_state_root: {}", old_state_root);
    println!("  new_state_root: {}", new_state_root);
    println!("  leaf: {}", leaf);
    println!("  leaf_index: {}", leaf_index);
    println!("=============================================");
    let circuit = CrossChainCircuit {
        old_state_root,
        new_state_root,
        merkle_proof: merkle_proof.clone(),
        leaf,
        leaf_index,
    };
    let start = Instant::now();
    let proof = generate_gkr_proof(&circuit, &previous_proofs);
    let gen_time = start.elapsed();
    println!("⏱ Generation Time: {:.2}µs", gen_time.as_micros() as f64);
    println!("Proof Size: {} bytes", proof.bytes.len());
    let verify_start = Instant::now();
    let valid = verify_gkr_proof(&proof, &previous_proofs);
    let verify_time = verify_start.elapsed();
    println!("Verification Result: {}", valid);
    println!("⏱ Verification Time: {:.2}µs", verify_time.as_micros() as f64);
    assert!(valid, "CrossChain Test failed verification");
    println!("=============================================");
}