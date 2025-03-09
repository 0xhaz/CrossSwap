use crate::swap_circuit::SwapCircuitGKR;
use crate::liquidity_circuit::LiquidityCircuit;
use crate::cross_chain_circuit::CrossChainCircuit;
use crate::merkle_tree::MerkleTree;
use crate::proof::GKRProver;
use crate::libraries::types::{U256, U160, I256};
use expander_transcript::Proof;

pub fn create_large_batch_test() -> (Vec<Box<dyn GKRProver>>, Vec<Proof>) {
    let decimals = U256::from(10).pow(U256::from(18));
    let mut circuits: Vec<Box<dyn GKRProver>> = Vec::new();
    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }]; // Initial proof

    // Simulate 60 swap transactions
    for i in 0..160 {
        let amount = 1000 + (i % 10) * 500; // Vary amount: 1000 to 5500
        let zero_for_one = i % 2 == 0; // Alternate swap direction
        let circuit = create_swap_test(
            &format!("Swap Test Large {}", i),
            zero_for_one,
            amount,
            if zero_for_one { "7130534626283790383418955530240" } else { "7922816251426433759354395033600" },
            if zero_for_one { "7922816251426433759354395033600" } else { "7130534626283790383418955530240" },
            (5_500_000 + (i % 5) * 100_000) as u128, // Vary liquidity: 5.5M to 6M
            (3000 + (i % 3) * 100) as u32, // Vary fee: 3000 to 3200
            vec![],
        );
        circuits.push(Box::new(circuit));
    }

    // Simulate 40 liquidity add/remove transactions
    for i in 0..140 {
        let delta = if i % 2 == 0 { 
            (2_000_000 + (i % 5) * 100_000) as i128 
        } else { 
            -((1_000_000 + (i % 5) * 100_000) as i128) 
        }; // Add 2M-2.4M, Remove 1M-1.4M
        let tick_lower = -200 + (i % 5) * 10; // Vary ticks: -200 to -160
        let tick_upper = 200 - (i % 5) * 10; // Vary ticks: 200 to 160
        let circuit = create_liquidity_test(
            &format!("Liquidity Test Large {}", i),
            0x1234 + i as u128, // Unique owner
            tick_lower,
            tick_upper,
            delta,
            1,
            [0u8; 32],
            "79228162514264337593543950336",
            vec![],
        );
        circuits.push(Box::new(circuit));
    }

    // Add a few cross-chain updates for variety (e.g., 50)
    let mut leaves = vec![1, 2, 3, 4, 5];
    for i in 0..50 {
        let new_leaf = 10 + i as u128;
        let index = i % leaves.len();
        let circuit = create_cross_chain_test(leaves.clone(), index, new_leaf);
        leaves[index] = new_leaf; // Update leaves for next iteration
        circuits.push(Box::new(circuit));
    }

    println!("Created large batch with {} circuits: 160 swaps, 140 liquidity, 50 cross-chain", circuits.len());
    (circuits, previous_proofs)
}

// Helper functions (copied from main.rs)
pub fn create_swap_test(
    name: &str,
    zero_for_one: bool,
    amount: i128,
    limit: &str,
    current: &str,
    liquidity: u128,
    fee: u32,
    hook_data: Vec<u8>,
) -> SwapCircuitGKR {
    let decimals = U256::from(10).pow(U256::from(18));
    SwapCircuitGKR {
        zero_for_one,
        amount_specified: I256::from(amount) * I256::from(decimals),
        sqrt_price_limit_x96: U160::from_dec_str(limit).unwrap(),
        sqrt_price_current_x96: U160::from_dec_str(current).unwrap(),
        liquidity,
        fee_pips: fee,
        hook_data,
    }
}

pub fn create_liquidity_test(
    name: &str,
    owner: u128,
    tick_lower: i32,
    tick_upper: i32,
    delta: i128,
    spacing: i32,
    salt: [u8; 32],
    sqrt_price: &str,
    hook_data: Vec<u8>,
) -> LiquidityCircuit {
    LiquidityCircuit {
        owner: U256::from(owner),
        tick_lower,
        tick_upper,
        liquidity_delta: delta,
        tick_spacing: spacing,
        salt,
        sqrt_price_current_x96: U256::from_dec_str(sqrt_price).unwrap(),
        hook_data,
    }
}

pub fn create_cross_chain_test(old_leaves: Vec<u128>, index: usize, new_leaf: u128) -> CrossChainCircuit {
    let old_leaves = old_leaves.into_iter().map(U256::from).collect::<Vec<_>>();
    let old_tree = MerkleTree::new(old_leaves.clone());
    let old_state_root = old_tree.get_root();
    let mut new_leaves = old_leaves;
    new_leaves[index] = U256::from(new_leaf);
    let new_tree = MerkleTree::new(new_leaves);
    let new_state_root = new_tree.get_root();
    let leaf = U256::from(new_leaf);
    let merkle_proof = new_tree.get_proof(index);
    CrossChainCircuit {
        old_state_root,
        new_state_root,
        merkle_proof,
        leaf,
        leaf_index: index,
    }
}