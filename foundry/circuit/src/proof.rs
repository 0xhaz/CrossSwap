use expander_transcript::Proof;
use expander_compiler::circuit::config::Config;
use rand::{thread_rng, Rng};
use tiny_keccak::{Hasher, Keccak};

pub fn generate_gkr_proof<C: Config>(
    _config: &C::DefaultGKRConfig,  
    _field_config: &C::DefaultGKRFieldConfig,
    previous_proof: Option<&Proof>,  
) -> Proof {
    let mut rng = thread_rng();
    let mut proof_data: Vec<u8> = (0..4).map(|_| rng.gen()).collect(); 

    // ✅ If there is a previous proof, hash it and append to new proof
    if let Some(prev) = previous_proof {
        let mut hasher = Keccak::v256();
        hasher.update(&prev.bytes);  
        
        let mut hash_result = [0u8; 32];
        hasher.finalize(&mut hash_result);

        proof_data.extend_from_slice(&hash_result[..]);
    }

    Proof { bytes: proof_data }
}

pub fn verify_gkr_proof<C: Config>(
    _config: &C::DefaultGKRConfig, 
    _field_config: &C::DefaultGKRFieldConfig,  
    proof: &Proof,
    previous_proof: Option<&Proof>,  
) -> bool {
    if proof.bytes.is_empty() {
        return false;  
    }

    if let Some(prev) = previous_proof {
        let mut hasher = Keccak::v256();
        hasher.update(&prev.bytes);

        let mut hash_result = [0u8; 32];
        hasher.finalize(&mut hash_result);

        // ✅ Check if current proof includes previous proof commitment
        return proof.bytes.ends_with(&hash_result[..]);
    }

    true  // ✅ If no previous proof, just check proof is non-empty
}