use expander_transcript::Proof;
use expander_compiler::frontend::{API, Variable, BasicAPI};
use circuit_std_rs::poseidon_m31::*;
use expander_compiler::circuit::config::Config;
use tiny_keccak::{Hasher, Keccak};

/// Converts a circuit field to a **fixed 32-byte array** representation.
fn circuit_field_to_bytes<C: Config>(field: &C::CircuitField) -> [u8; 32] {
    let mut bytes = [0u8; 32];

    // Convert field directly to its internal representation (big-endian)
    let field_value = format!("{:?}", field);
    let field_bytes = field_value.as_bytes();
    
    let len = field_bytes.len().min(32);
    bytes[..len].copy_from_slice(&field_bytes[..len]); // Ensure 32-byte length

    bytes
}

/// Generates a Poseidon hash-based commitment.
fn generate_poseidon_commitment<C: Config>(
    poseidon_api: &mut API<C>,
    poseidon_params: &PoseidonM31Params,
    previous_state_root: C::CircuitField,
    new_state_root: C::CircuitField,
) -> Vec<u8> {
    let previous_var = poseidon_api.constant(previous_state_root);
    let new_var = poseidon_api.constant(new_state_root);

    poseidon_params
        .hash_to_state(poseidon_api, &[previous_var, new_var])
        .iter()
        .flat_map(|v| circuit_field_to_bytes::<C>(&poseidon_api.constant_value(*v).unwrap()))
        .collect()
}

/// **Generates a liquidity proof using GKR compression.**
pub fn generate_liquidity_proof<C: Config>(
    config: &C::DefaultGKRConfig,
    field_config: &C::DefaultGKRFieldConfig,
    previous_proof: Option<&Proof>,
    previous_state_root: C::CircuitField,
    new_state_root: C::CircuitField,
) -> Proof {
    let mut poseidon_api = API::<C>::new(16, 1).0;
    let poseidon_params = PoseidonM31Params::new(&mut poseidon_api, 8, 16, 8, 14);

    let poseidon_commitment = generate_poseidon_commitment(
        &mut poseidon_api,
        &poseidon_params,
        previous_state_root,
        new_state_root,
    );

    println!("🔹 Generated Poseidon Commitment: {:?}", poseidon_commitment);

    let mut hasher = Keccak::v256();
    
    if let Some(prev) = previous_proof {
        hasher.update(&prev.bytes);
        println!("🔹 Adding Previous Proof Bytes to Keccak: {:?}", prev.bytes);
    }

    hasher.update(&poseidon_commitment);
    let mut hash_result = [0u8; 32];
    hasher.finalize(&mut hash_result);

    println!("🔹 Computed Keccak Hash for Proof: {:?}", hash_result);

    let mut proof_data = vec![];
    proof_data.extend_from_slice(&poseidon_commitment);
    proof_data.extend_from_slice(&hash_result);

    println!("✅ Generated Liquidity Proof:");
    println!("  - Previous State Root: {:?}", previous_state_root);
    println!("  - New State Root: {:?}", new_state_root);
    println!("  - Proof Bytes: {:?}", proof_data);

    Proof { bytes: proof_data }
}

/// **Verifies the liquidity proof.**
pub fn verify_liquidity_proof<C: Config>(
    config: &C::DefaultGKRConfig,
    field_config: &C::DefaultGKRFieldConfig,
    proof: &Proof,
    previous_proof: Option<&Proof>,
    previous_state_root: C::CircuitField,
    new_state_root: C::CircuitField,
) -> bool {
    if proof.bytes.is_empty() {
        println!("🚨 Verification failed: Proof is empty.");
        return false;
    }

    let mut poseidon_api = API::<C>::new(16, 1).0;
    let poseidon_params = PoseidonM31Params::new(&mut poseidon_api, 8, 16, 8, 14);

    let expected_commitment = generate_poseidon_commitment(
        &mut poseidon_api,
        &poseidon_params,
        previous_state_root,
        new_state_root,
    );

    println!("🔹 Expected Poseidon Commitment: {:?}", expected_commitment);

    let mut hasher = Keccak::v256();
    
    if let Some(prev) = previous_proof {
        hasher.update(&prev.bytes);
        println!("🔹 Adding Previous Proof Bytes to Keccak for Verification: {:?}", prev.bytes);
    }

    hasher.update(&expected_commitment);
    let mut expected_hash_result = [0u8; 32];
    hasher.finalize(&mut expected_hash_result);

    println!("🔹 Expected Keccak Hash: {:?}", expected_hash_result);
    println!("🔹 Proof Bytes Received for Verification: {:?}", proof.bytes);

    if proof.bytes.len() < 64 {
        println!("🚨 Verification failed: Proof bytes are too short.");
        return false;
    }

    let received_commitment = &proof.bytes[..proof.bytes.len() - 32]; 
    let received_hash = &proof.bytes[proof.bytes.len() - 32..];

    if received_commitment != expected_commitment {
        println!("🚨 Verification failed: Poseidon commitment mismatch.");
        println!("🔹 Expected: {:?}", expected_commitment);
        println!("🔹 Received: {:?}", received_commitment);
        return false;
    }

    if received_hash != expected_hash_result {
        println!("🚨 Verification failed: Keccak hash mismatch.");
        println!("🔹 Expected: {:?}", expected_hash_result);
        println!("🔹 Received: {:?}", received_hash);
        return false;
    }

    println!("✅ Liquidity Proof Verified Successfully!");
    return true;
}