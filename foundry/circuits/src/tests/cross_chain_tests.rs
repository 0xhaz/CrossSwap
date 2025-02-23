#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::{generate_liquidity_proof, verify_liquidity_proof};
    use expander_compiler::circuit::config::M31Config;
    use expander_transcript::Proof;

    #[test]
fn test_liquidity_proof_generation() {
    let user_balance = 100;
    let liquidity_added = 50;
    let pool_total_liquidity = 1000;
    let expected_new_total = user_balance + liquidity_added;  

    let proof = generate_liquidity_proof(user_balance, liquidity_added, pool_total_liquidity, expected_new_total);
    
    // println!("🔹 Generated Proof Bytes: {:02x?}", proof.bytes);
    assert!(!proof.bytes.is_empty(), "❌ Liquidity proof should not be empty!");
}

#[test]
fn test_liquidity_proof_verification() {
    let user_balance = 100;
    let liquidity_added = 50;
    let pool_total_liquidity = 1000;
    let expected_new_total = user_balance + liquidity_added;  

    let proof = generate_liquidity_proof(user_balance, liquidity_added, pool_total_liquidity, expected_new_total);
    
    // println!("🔹 Verifying Proof Bytes: {:02x?}", proof.bytes);

    let is_valid = verify_liquidity_proof(&proof, user_balance, liquidity_added, pool_total_liquidity, expected_new_total);
    assert!(is_valid, "❌ Liquidity proof verification failed!");
}
}