use crate::liquidity_circuit::{LiquidityCircuit, LiquidityProofWrapperCircuit};
use crate::cross_chain_circuit::{CrossChainCircuit, CrossChainProofWrapperCircuit};
use expander_compiler::frontend::{
    Define, Config, Variable, BasicAPI, API as RootBuilder,
};
use expander_compiler::circuit::config::{BN254Config};
use crate::poseidon_bn254::Poseidon;
use expander_transcript::Proof;
use rand::{thread_rng};
use crate::swap_circuit::{SwapCircuitGKR, ProofWrapperCircuit};
use ark_groth16::{Groth16, Proof as Groth16Proof, VerifyingKey};
use ark_bn254::{Bn254, Fr as ArkFr, G1Affine, G2Affine};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_ff::PrimeField as ArkPrimeField;
use ark_snark::SNARK;
use std::time::Instant;
use halo2curves::ff::PrimeField as HaloPrimeField;
use primitive_types::U256;
use expander_compiler::frontend::BN254;
use arith::FieldForECC;
use ethnum::U256 as EthnumU256;

pub fn u256_to_bn254(u: U256) -> BN254 {
    let ethnum_u = primitive_to_ethnum_u256(u);
    BN254::from_u256(ethnum_u)
}

fn primitive_to_ethnum_u256(u: primitive_types::U256) -> EthnumU256 {
    let mut bytes = [0u8; 32];
    u.to_little_endian(&mut bytes);
    EthnumU256::from_le_bytes(bytes)
}

pub fn variable_to_u8<C: Config, B: BasicAPI<C>>(api: &mut B, v: &Variable) -> u8
where
    C::CircuitField: FieldForECC + HaloPrimeField,
{
    match api.constant_value(*v) {
        Some(value) => {
            let u256_value = value.to_u256();
            let bytes = u256_value.to_le_bytes();
            bytes[0]
        }
        None => panic!("Failed to extract u8 from Variable"),
    }
}

pub fn generate_gkr_proof(previous_proofs: &[Proof]) -> Proof {
    let start_time = Instant::now();
    let (_api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(0, 0);
    let poseidon = Poseidon::new(8, 1, 1); // Simplified: 8 rounds, 1 capacity, 1 rate
    let mut proof_bytes = Vec::new();

    if previous_proofs.is_empty() {
        return Proof { bytes: vec![0xAA; 32] };
    }

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
            println!("No valid constants in proof, using fallback");
            proof_bytes.extend(vec![0xEE; 32]);
            continue;
        }

        let prev_hash = poseidon.hash(&proof_constants).unwrap();
        let mut hash_bytes = [0u8; 32];
        prev_hash.serialize_compressed(&mut hash_bytes[..]).unwrap();
        proof_bytes.extend(&hash_bytes);
    }

    if proof_bytes.is_empty() {
        println!("No proof bytes generated, using fallback");
        proof_bytes.extend(vec![0xAB; 32]);
    }

    println!("⏱ GKR Proof Generation Time: {:.2?}", start_time.elapsed());
    Proof { bytes: proof_bytes }
}

pub fn verify_gkr_proof(proof: &Proof, previous_proofs: &[Proof]) -> bool {
    if proof.bytes.is_empty() {
        println!("Verification failed: Proof is empty");
        return false;
    }

    let (_api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(0, 0);
    let poseidon = Poseidon::new(8, 1, 1); // Match generation parameters
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
                println!("No valid constants in proof, using fallback");
                expected_hash_bytes.extend(vec![0xDE; 32]);
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
    input_token: U256,
    output_token: U256,
    liquidity: U256,
    slippage_tolerance: U256,
    expected_output: U256,
    user_balance: U256,
    pool_total_liquidity: U256,
    previous_proofs: &[Proof],
    old_state_root: U256,
    new_state_root: U256,
    merkle_proof: Vec<U256>,
    leaf: U256,
) -> (Proof, [Vec<u8>; 3], Vec<U256>, VerifyingKey<Bn254>) {
    let rng = &mut thread_rng();
    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(5, 0);

    let gkr_proof = if input_type == 1 {
        let circuit = LiquidityCircuit {
            user_balance,
            liquidity_added: liquidity,
            pool_total_liquidity,
            expected_new_total: pool_total_liquidity + liquidity,
        };
        circuit.define(&mut api);
        generate_gkr_proof(previous_proofs)
    } else if input_type == 2 {
        let gkr_circuit = CrossChainCircuit {
            old_state_root: api.constant(u256_to_bn254(old_state_root)),
            new_state_root: api.constant(u256_to_bn254(new_state_root)),
            merkle_proof: merkle_proof.iter().map(|v| api.constant(u256_to_bn254(*v))).collect(),
            leaf: api.constant(u256_to_bn254(leaf)),
        };
        gkr_circuit.define(&mut api);
        let mut bytes = [0u8; 32];
        new_state_root.to_little_endian(&mut bytes);
        Proof { bytes: bytes.to_vec() }
    } else {
        let gkr_circuit = SwapCircuitGKR {
            input_token,
            output_token,
            liquidity,
            slippage_tolerance,
            expected_output,
        };
        gkr_circuit.define(&mut api);
        generate_gkr_proof(previous_proofs)
    };

    println!("Generating Groth16 params with proof_hash: {:?}", gkr_proof.bytes);
    let params = if input_type == 1 {
        let wrapper_circuit = LiquidityProofWrapperCircuit {
            proof_hash: gkr_proof.bytes.clone(),
        };
        Groth16::<Bn254>::generate_random_parameters_with_reduction(wrapper_circuit, rng).unwrap()
    } else if input_type == 2 {
        let wrapper_circuit = CrossChainProofWrapperCircuit {
            proof_hash: gkr_proof.bytes.clone(),
        };
        Groth16::<Bn254>::generate_random_parameters_with_reduction(wrapper_circuit, rng).unwrap()
    } else {
        let wrapper_circuit = ProofWrapperCircuit {
            proof_hash: gkr_proof.bytes.clone(),
        };
        Groth16::<Bn254>::generate_random_parameters_with_reduction(wrapper_circuit, rng).unwrap()
    };

    println!("Creating Groth16 proof with proof_hash: {:?}", gkr_proof.bytes);
    let proof = if input_type == 1 {
        let wrapper_circuit = LiquidityProofWrapperCircuit {
            proof_hash: gkr_proof.bytes.clone(),
        };
        Groth16::<Bn254>::create_random_proof_with_reduction(wrapper_circuit, &params, rng).unwrap()
    } else if input_type == 2 {
        let wrapper_circuit = CrossChainProofWrapperCircuit {
            proof_hash: gkr_proof.bytes.clone(),
        };
        Groth16::<Bn254>::create_random_proof_with_reduction(wrapper_circuit, &params, rng).unwrap()
    } else {
        let wrapper_circuit = ProofWrapperCircuit {
            proof_hash: gkr_proof.bytes.clone(),
        };
        Groth16::<Bn254>::create_random_proof_with_reduction(wrapper_circuit, &params, rng).unwrap()
    };

    let mut proof_a_bytes = Vec::new();
    let mut proof_b_bytes = Vec::new();
    let mut proof_c_bytes = Vec::new();
    proof.a.serialize_uncompressed(&mut proof_a_bytes).unwrap();
    proof.b.serialize_uncompressed(&mut proof_b_bytes).unwrap();
    proof.c.serialize_uncompressed(&mut proof_c_bytes).unwrap();

    let solidity_proof = [proof_a_bytes, proof_b_bytes, proof_c_bytes];
    let mut proof_hash_bytes = [0u8; 32];
    proof_hash_bytes.copy_from_slice(&gkr_proof.bytes);
    let public_inputs = vec![U256::from_little_endian(&proof_hash_bytes)];

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
            println!("Verifying Key: {:?}", verifying_key);
            println!("Proof: {:?}", groth16_proof);
            false
        }
    }
}