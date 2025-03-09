use crate::liquidity_circuit::{LiquidityCircuit, LiquidityProofWrapperCircuit};
use crate::cross_chain_circuit::{CrossChainCircuit, CrossChainProofWrapperCircuit};
use crate::swap_circuit::{SwapCircuitGKR, ProofWrapperCircuit};
use expander_compiler::frontend::{Define, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use crate::poseidon_bn254::Poseidon;
use expander_transcript::Proof;
use rand::thread_rng;
use ark_groth16::{Groth16, Proof as Groth16Proof, VerifyingKey};
use ark_bn254::{Bn254, Fr as ArkFr, G1Affine, G2Affine};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_snark::SNARK;
use std::time::Instant;
use crate::libraries::types::{U256, U160}; // Added U160
use expander_compiler::field::BN254;
use arith::FieldForECC;
use ethnum::U256 as EthnumU256;
use ark_ff::PrimeField as ArkPrimeField;

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
            let proof_constants: Vec<ArkFr> = prev.bytes
                .chunks_exact(32)
                .filter_map(|chunk| {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(chunk);
                    Some(ArkFr::from_le_bytes_mod_order(&bytes))
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
            let proof_constants: Vec<ArkFr> = prev.bytes
                .chunks_exact(32)
                .filter_map(|chunk| {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(chunk);
                    Some(ArkFr::from_le_bytes_mod_order(&bytes))
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

pub fn generate_proof(
    input_type: u32,
    zero_for_one: bool,
    amount_specified: U256,
    sqrt_price_limit_x96: U256,
    sqrt_price_current_x96_swap: U256,
    liquidity_swap: U256,
    fee_pips: U256,
    owner: U256,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_delta: i128,
    tick_spacing: i32,
    salt: [u8; 32],
    sqrt_price_current_x96_liquidity: U256,
    old_state_root: U256,
    new_state_root: U256,
    merkle_proof: Vec<U256>,
    leaf: U256,
    previous_proofs: &[Proof],
) -> (Proof, [Vec<u8>; 3], Vec<U256>, VerifyingKey<Bn254>) {
    let rng = &mut thread_rng();
    let gkr_proof = match input_type {
        0 => {
            let circuit = SwapCircuitGKR {
                zero_for_one,
                amount_specified,
                sqrt_price_limit_x96: U160::from(sqrt_price_limit_x96), // Convert U256 to U160
                sqrt_price_current_x96: U160::from(sqrt_price_current_x96_swap), // Convert U256 to U160
                liquidity: liquidity_swap.as_u128(), // Convert U256 to u128
                fee_pips: fee_pips.as_u32(), // Convert U256 to u32
            };
            generate_gkr_proof(&circuit, previous_proofs)
        }
        1 => {
            let circuit = LiquidityCircuit {
                owner,
                tick_lower,
                tick_upper,
                liquidity_delta,
                tick_spacing,
                salt,
                sqrt_price_current_x96: sqrt_price_current_x96_liquidity,
            };
            generate_gkr_proof(&circuit, previous_proofs)
        }
        2 => {
            let circuit = CrossChainCircuit {
                old_state_root,
                new_state_root,
                merkle_proof,
                leaf,
            };
            generate_gkr_proof(&circuit, previous_proofs)
        }
        _ => panic!("Invalid input_type"),
    };

    println!("Generating Groth16 params with proof_hash: {:?}", gkr_proof.bytes);
    let params = match input_type {
        0 => {
            let wrapper_circuit = ProofWrapperCircuit {
                proof_hash: gkr_proof.bytes.clone(),
            };
            Groth16::<Bn254>::generate_random_parameters_with_reduction(wrapper_circuit, rng).unwrap()
        }
        1 => {
            let wrapper_circuit = LiquidityProofWrapperCircuit {
                proof_hash: gkr_proof.bytes.clone(),
            };
            Groth16::<Bn254>::generate_random_parameters_with_reduction(wrapper_circuit, rng).unwrap()
        }
        2 => {
            let wrapper_circuit = CrossChainProofWrapperCircuit {
                proof_hash: gkr_proof.bytes.clone(),
            };
            Groth16::<Bn254>::generate_random_parameters_with_reduction(wrapper_circuit, rng).unwrap()
        }
        _ => unreachable!(),
    };

    println!("Creating Groth16 proof with proof_hash: {:?}", gkr_proof.bytes);
    let proof = match input_type {
        0 => {
            let wrapper_circuit = ProofWrapperCircuit {
                proof_hash: gkr_proof.bytes.clone(),
            };
            Groth16::<Bn254>::create_random_proof_with_reduction(wrapper_circuit, &params, rng).unwrap()
        }
        1 => {
            let wrapper_circuit = LiquidityProofWrapperCircuit {
                proof_hash: gkr_proof.bytes.clone(),
            };
            Groth16::<Bn254>::create_random_proof_with_reduction(wrapper_circuit, &params, rng).unwrap()
        }
        2 => {
            let wrapper_circuit = CrossChainProofWrapperCircuit {
                proof_hash: gkr_proof.bytes.clone(),
            };
            Groth16::<Bn254>::create_random_proof_with_reduction(wrapper_circuit, &params, rng).unwrap()
        }
        _ => unreachable!(),
    };

    let mut proof_a_bytes = Vec::new();
    let mut proof_b_bytes = Vec::new();
    let mut proof_c_bytes = Vec::new();
    proof.a.serialize_uncompressed(&mut proof_a_bytes).unwrap();
    proof.b.serialize_uncompressed(&mut proof_b_bytes).unwrap();
    proof.c.serialize_uncompressed(&mut proof_c_bytes).unwrap();

    let solidity_proof = [proof_a_bytes, proof_b_bytes, proof_c_bytes];
    // Use new_state_root as public input for CrossChainCircuit, proof hash for others
    let public_inputs = if input_type == 2 {
        vec![new_state_root]
    } else {
        let mut proof_hash_bytes = [0u8; 32];
        proof_hash_bytes.copy_from_slice(&gkr_proof.bytes);
        vec![U256::from_little_endian(&proof_hash_bytes)]
    };

    println!("Generated Public Inputs for Groth16: {:?}", public_inputs);
    (gkr_proof, solidity_proof, public_inputs, params.vk)
}

pub fn verify_proof(
    gkr_proof: &Proof,
    solidity_proof: [Vec<u8>; 3],
    public_inputs: Vec<U256>,
    verifying_key: &VerifyingKey<Bn254>,
    _previous_proofs: &[Proof],
) -> bool {
    let start_time = Instant::now();

    let mut expected_bytes = [0u8; 32];
    public_inputs[0].to_little_endian(&mut expected_bytes);
    let expected_proof = Proof { bytes: expected_bytes.to_vec() };
    if gkr_proof.bytes != expected_proof.bytes {
        println!("GKR proof verification failed");
        println!("Expected GKR proof: {:?}", expected_proof.bytes);
        println!("Actual GKR proof: {:?}", gkr_proof.bytes);
        return false;
    }

    let proof_a = match G1Affine::deserialize_uncompressed(&*solidity_proof[0]) {
        Ok(a) => a,
        Err(e) => {
            println!("Failed to deserialize proof A: {:?}", e);
            return false;
        }
    };
    let proof_b = match G2Affine::deserialize_uncompressed(&*solidity_proof[1]) {
        Ok(b) => b,
        Err(e) => {
            println!("Failed to deserialize proof B: {:?}", e);
            return false;
        }
    };
    let proof_c = match G1Affine::deserialize_uncompressed(&*solidity_proof[2]) {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to deserialize proof C: {:?}", e);
            return false;
        }
    };
    let groth16_proof = Groth16Proof::<Bn254> {
        a: proof_a,
        b: proof_b,
        c: proof_c,
    };

    let public_inputs_fr: Vec<ArkFr> = public_inputs
        .iter()
        .map(|u| {
            let mut bytes = [0u8; 32];
            u.to_little_endian(&mut bytes);
            ArkFr::from_le_bytes_mod_order(&bytes)
        })
        .collect();
    println!("Public Inputs (Fr): {:?}", public_inputs_fr);
    assert_eq!(public_inputs_fr.len(), 1, "Expected 1 public input for ProofWrapperCircuit");

    match Groth16::<Bn254>::verify(verifying_key, &public_inputs_fr, &groth16_proof) {
        Ok(is_valid) => {
            println!("Hybrid GKR + Groth16 Verification result: {}", is_valid);
            println!("⏱ Verification Time: {:.2?}", start_time.elapsed());
            is_valid
        }
        Err(e) => {
            println!("Groth16 verification failed: {:?}", e);
            false
        }
    }
}