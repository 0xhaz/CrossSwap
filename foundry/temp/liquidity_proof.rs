use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{Circuit, ConstraintSystem, Error},
    poly::commitment::Params,
};
use halo2curves::bn256::{Fr, G1Affine};
use halo2curves::poseidon::{Poseidon, Hash};
use rand_core::OsRng;
use serde_json::json;
use std::fs::File;
use std::io::Write;

/// **Liquidity Circuit for Proof Generation**
#[derive(Default)]
struct LiquidityCircuit {
    previous_state: Value<Fr>,
    new_state: Value<Fr>,
}

impl Circuit<Fr> for LiquidityCircuit {
    type Config = ();

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        meta.enable_equality();
    }

    fn synthesize(&self, _: Self::Config, mut layouter: impl Layouter<Fr>) -> Result<(), Error> {
        let prev_state_cell = layouter.assign_region(
            || "Previous State",
            |mut region| {
                region.assign_advice(|| "prev_state", 0, || self.previous_state)
            }
        )?;

        let new_state_cell = layouter.assign_region(
            || "New State",
            |mut region| {
                region.assign_advice(|| "new_state", 1, || self.new_state)
            }
        )?;

        layouter.assign_region(
            || "Verify Poseidon Hash",
            |mut region| {
                let mut hasher = Poseidon::new();
                hasher.update(prev_state_cell.value().unwrap());
                hasher.update(new_state_cell.value().unwrap());

                let hash = hasher.finalize();
                region.assign_advice(|| "poseidon_commitment", 2, || Value::known(hash))
            }
        )?;

        Ok(())
    }
}

/// **Generates a zkSNARK proof using Halo2**
fn generate_liquidity_proof(
    previous_state: Fr,
    new_state: Fr,
) -> Vec<u8> {
    let circuit = LiquidityCircuit {
        previous_state: Value::known(previous_state),
        new_state: Value::known(new_state),
    };

    let params = Params::<Fr>::new(8);
    let vk = halo2_proofs::plonk::keygen_vk(&params, &circuit).unwrap();
    let pk = halo2_proofs::plonk::keygen_pk(&params, vk, &circuit).unwrap();

    let mut transcript = halo2_proofs::transcript::Blake2bWrite::init(Vec::new());
    halo2_proofs::plonk::create_proof(&params, &pk, &[circuit], &mut transcript).unwrap();
    
    transcript.finalize()
}

/// **Exports proof to a Solidity-compatible format**
fn export_proof(proof: Vec<u8>) {
    let proof_json = json!({
        "proof": hex::encode(proof),
    });

    let mut file = File::create("proof.json").unwrap();
    file.write_all(proof_json.to_string().as_bytes()).unwrap();
    println!("✅ Proof exported to proof.json");
}

/// **Main Execution**
fn main() {
    let proof = generate_liquidity_proof(Fr::from(123456), Fr::from(7891011));
    export_proof(proof);
}