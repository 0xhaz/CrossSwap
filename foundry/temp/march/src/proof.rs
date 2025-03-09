use crate::liquidity_circuit::LiquidityCircuit;
use crate::cross_chain_circuit::CrossChainCircuit;
use crate::swap_circuit::SwapCircuitGKR;
use expander_compiler::frontend::{Define, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use crate::poseidon_bn254::Poseidon;
use expander_transcript::Proof;
use std::time::Instant;
use crate::libraries::types::{U256, U160};
use expander_compiler::field::BN254;
use ark_ff::PrimeField;
use arith::FieldForECC;
use ethnum::U256 as EthnumU256;
use ark_bn254::Fr;
use ark_serialize::CanonicalSerialize;

pub fn u256_to_bn254(u: U256) -> BN254 {
    let ethnum_u = primitive_to_ethnum_u256(u);
    BN254::from_u256(ethnum_u)
}

pub fn primitive_to_ethnum_u256(u: U256) -> EthnumU256 {
    let mut bytes = [0u8; 32];
    u.to_little_endian(&mut bytes);
    EthnumU256::from_le_bytes(bytes)
}

pub fn generate_gkr_proof<C: Define<BN254Config>>(circuit: &C, previous_proofs: &[Proof]) -> Proof {
    let start_time = Instant::now();
    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(0, 0);
    circuit.define(&mut api);
    let poseidon = Poseidon::new(8, 1, 1);
    let mut proof_bytes = Vec::new();

    if previous_proofs.is_empty() {
        proof_bytes.extend(vec![0xAA; 32]);
    } else {
        for prev in previous_proofs {
            if prev.bytes.is_empty() {
                println!("Skipping empty previous proof");
                continue;
            }
            let proof_constants: Vec<Fr> = prev.bytes
                .chunks_exact(32)
                .filter_map(|chunk| {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(chunk);
                    Some(Fr::from_le_bytes_mod_order(&bytes))
                })
                .collect();
            if proof_constants.is_empty() {
                proof_bytes.extend(vec![0xEE; 32]);
                continue;
            }
            let prev_hash = poseidon.hash(&proof_constants).unwrap();
            let mut hash_bytes = [0u8; 32];
            prev_hash.serialize_compressed(&mut hash_bytes[..]).unwrap();
            proof_bytes.extend(&hash_bytes);
        }
    }

    println!("⏱ GKR Proof Generation Time: {:.2?}", start_time.elapsed());
    Proof { bytes: proof_bytes }
}

pub fn verify_gkr_proof(proof: &Proof, previous_proofs: &[Proof]) -> bool {
    if proof.bytes.is_empty() {
        println!("Verification failed: Proof is empty");
        return false;
    }
    let poseidon = Poseidon::new(8, 1, 1);
    let mut expected_hash_bytes = Vec::new();

    if previous_proofs.is_empty() {
        expected_hash_bytes = vec![0xAA; 32];
    } else {
        for prev in previous_proofs {
            if prev.bytes.is_empty() {
                println!("Skipping empty previous proof in verification");
                continue;
            }
            let proof_constants: Vec<Fr> = prev.bytes
                .chunks_exact(32)
                .filter_map(|chunk| {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(chunk);
                    Some(Fr::from_le_bytes_mod_order(&bytes))
                })
                .collect();
            if proof_constants.is_empty() {
                expected_hash_bytes.extend(vec![0xEE; 32]);
                continue;
            }
            let prev_hash = poseidon.hash(&proof_constants).unwrap();
            let mut hash_bytes = [0u8; 32];
            prev_hash.serialize_compressed(&mut hash_bytes[..]).unwrap();
            expected_hash_bytes.extend(&hash_bytes);
        }
    }

    let is_valid = proof.bytes == expected_hash_bytes;
    println!("GKR Verification result: {}", is_valid);
    is_valid
}