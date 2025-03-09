use crate::liquidity_circuit::LiquidityCircuit;
use crate::swap_circuit::SwapCircuitGKR;
use expander_compiler::frontend::{Define, Config, BasicAPI, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use crate::poseidon_bn254::Poseidon;
use expander_transcript::Proof;
use std::time::Instant;
use crate::libraries::types::{U256, U160, I256};
use expander_compiler::field::BN254;
use ark_ff::PrimeField;
use arith::FieldForECC;
use ethnum::U256 as EthnumU256;
use ark_bn254::Fr;
use ark_serialize::CanonicalSerialize;
use std::any::Any;

pub fn u256_to_bn254(u: U256) -> BN254 {
    let ethnum_u = primitive_to_ethnum_u256(u);
    BN254::from_u256(ethnum_u)
}

pub fn primitive_to_ethnum_u256(u: U256) -> EthnumU256 {
    let mut bytes = [0u8; 32];
    u.to_little_endian(&mut bytes);
    EthnumU256::from_le_bytes(bytes)
}

#[derive(Clone)]
pub struct VerifierCircuitGKR {
    proof: Proof,
    aggregated_proofs: Vec<Proof>,
    public_outputs: Vec<U256>,
}

impl<C: Config> Define<C> for VerifierCircuitGKR
where
    C::CircuitField: From<BN254> + PartialOrd + Clone + FieldForECC,
{
    fn define(&self, builder: &mut RootBuilder<C>) {
        let poseidon = Poseidon::new(8, 1, 2);
        let aggregated_proof = aggregate_proofs(&poseidon, &self.aggregated_proofs);

        let proof_hash = builder.constant(u256_to_bn254(U256::from_little_endian(&self.proof.bytes)));
        let expected_hash = builder.constant(u256_to_bn254(U256::from_little_endian(&aggregated_proof.bytes)));
        builder.assert_is_equal(proof_hash, expected_hash);
    }
}

pub trait GKRProver: Define<BN254Config> + CircuitPublicOutputs {}

pub trait CircuitPublicOutputs {
    fn as_any(&self) -> &dyn Any;
    fn get_public_outputs(&self) -> Vec<U256>;
}

fn hash_to_proof(poseidon: &Poseidon, bytes: Vec<u8>) -> Proof {
    let constants: Vec<Fr> = bytes
        .chunks_exact(32)
        .filter_map(|chunk| {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(chunk);
            Some(Fr::from_le_bytes_mod_order(&bytes))
        })
        .collect();
    let hash = poseidon.hash(&constants).expect("Poseidon hash failed");
    let mut hash_bytes = [0u8; 32];
    hash.serialize_compressed(&mut hash_bytes[..]).expect("Serialization failed");
    Proof { bytes: hash_bytes.to_vec() }
}

fn aggregate_proofs(poseidon: &Poseidon, proofs: &[Proof]) -> Proof {
    let mut aggregated_bytes = proofs[0].bytes.clone();
    for proof in &proofs[1..] {
        aggregated_bytes.extend(&proof.bytes);
        aggregated_bytes = hash_to_proof(poseidon, aggregated_bytes).bytes;
    }
    println!("Aggregated Proof: 0x{}", hex::encode(&aggregated_bytes));
    Proof { bytes: aggregated_bytes }
}

pub fn generate_gkr_proof(circuits: &[&dyn GKRProver], previous_proofs: &[Proof]) -> (Proof, Vec<Proof>) {
    let start_time = Instant::now();
    let poseidon = Poseidon::new(56, 1, 2); // Total rounds = 56
    let mut proofs: Vec<Proof> = Vec::with_capacity(circuits.len());
    let mut all_public_outputs: Vec<Vec<U256>> = Vec::with_capacity(circuits.len());

    let initial_proof = if previous_proofs.is_empty() {
        Proof {
            bytes: hex::decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap()
        }
    } else {
        aggregate_proofs(&poseidon, previous_proofs)
    };
    println!("Initial Proof: 0x{}", hex::encode(&initial_proof.bytes));

    for (i, circuit) in circuits.iter().enumerate() {
        let (mut api, _, _) = RootBuilder::<BN254Config>::new(0, 2);
        circuit.define(&mut api);
        let public_outputs = circuit.get_public_outputs();
        if public_outputs.len() < 2 {
            panic!("Circuit {} must provide at least 2 public outputs", i);
        }
        all_public_outputs.push(public_outputs.clone());

        let mut amount0_bytes = [0u8; 32];
        let mut amount1_bytes = [0u8; 32];
        public_outputs[0].to_little_endian(&mut amount0_bytes);
        public_outputs[1].to_little_endian(&mut amount1_bytes);

        // Compute intermediate hash
        let prev_proof = if i > 0 { &proofs[i - 1] } else { &initial_proof };
        let inputs1 = [
            Fr::from_le_bytes_mod_order(&prev_proof.bytes),
            Fr::from_le_bytes_mod_order(&amount0_bytes),
        ];
        let intermediate_proof = poseidon.hash_to_bytes(&inputs1).expect("Poseidon hash failed");
        println!("Circuit {} - intermediate_proof: 0x{}", i, hex::encode(&intermediate_proof));

        // Compute final proof
        let inputs2 = [
            Fr::from_le_bytes_mod_order(&intermediate_proof),
            Fr::from_le_bytes_mod_order(&amount1_bytes),
        ];
        let circuit_proof = poseidon.hash_to_bytes(&inputs2).expect("Poseidon hash failed");
        proofs.push(Proof { bytes: circuit_proof });
    }

    let final_proof = if proofs.len() > 1 {
        aggregate_proofs(&poseidon, &proofs)
    } else {
        proofs[0].clone()
    };

    println!("⏱ GKR Proof Generation Time: {:.2?}", start_time.elapsed());
    (final_proof, proofs)
}

pub fn verify_gkr_proof(proof: &Proof, all_proofs: &[Proof]) -> bool {
    if proof.bytes.is_empty() || all_proofs.is_empty() {
        println!("Verification failed: Proof or all_proofs is empty");
        return false;
    }

    let poseidon = Poseidon::new(8, 1, 2);
    let aggregated_proof = aggregate_proofs(&poseidon, all_proofs);
    let is_valid = proof.bytes == aggregated_proof.bytes;

    println!("GKR Verification result: {}", is_valid);
    is_valid
}