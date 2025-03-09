use expander_compiler::frontend::{Define, Config, BasicAPI, API as RootBuilder};
use expander_compiler::circuit::config::BN254Config;
use expander_transcript::Proof;
use crate::proof::{verify_gkr_proof, u256_to_bn254};
use expander_compiler::frontend::BN254;
use arith::FieldForECC;
use halo2curves::ff::PrimeField;
use primitive_types::U256;
use ark_bn254::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable as R1CSVariable};
use ark_ff::PrimeField as ArkPrimeField;
use ark_relations::lc;
use ark_std::One;

pub struct LiquidityCircuit {
    pub user_balance: U256,
    pub liquidity_added: U256,
    pub pool_total_liquidity: U256,
    pub expected_new_total: U256,
}

impl<C: Config> Define<C> for LiquidityCircuit
where
    C::CircuitField: FieldForECC + PartialOrd + Clone + From<BN254>,
{
    fn define(&self, builder: &mut RootBuilder<C>) {
        let _user_balance_bn254 = builder.constant(u256_to_bn254(self.user_balance));
        let liquidity_added_bn254 = builder.constant(u256_to_bn254(self.liquidity_added));
        let pool_total_liquidity_bn254 = builder.constant(u256_to_bn254(self.pool_total_liquidity));
        let expected_new_total_bn254 = builder.constant(u256_to_bn254(self.expected_new_total));

        let new_pool_total = builder.add(pool_total_liquidity_bn254, liquidity_added_bn254);
        let diff = builder.sub(new_pool_total, expected_new_total_bn254);
        builder.assert_is_zero(diff);
    }
}

#[derive(Clone)]
pub struct LiquidityProofWrapperCircuit {
    pub proof_hash: Vec<u8>,
}

impl ConstraintSynthesizer<Fr> for LiquidityProofWrapperCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let mut proof_hash_bytes = [0u8; 32];
        proof_hash_bytes.copy_from_slice(&self.proof_hash);
        let proof_hash_u256 = U256::from_little_endian(&proof_hash_bytes);
        let mut bytes = [0u8; 32];
        proof_hash_u256.to_little_endian(&mut bytes);
        let proof_hash_fr = Fr::from_le_bytes_mod_order(&bytes);

        let proof_hash_var = cs.new_input_variable(|| Ok(proof_hash_fr))?;
        cs.enforce_constraint(
            lc!() + (Fr::one(), proof_hash_var),
            lc!() + (Fr::one(), R1CSVariable::One),
            lc!() + (Fr::one(), proof_hash_var)
        )?;
        Ok(())
    }
}

pub fn generate_liquidity_proof(
    user_balance: U256,
    liquidity_added: U256,
    pool_total_liquidity: U256,
    expected_new_total: U256,
) -> Proof {
    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(4, 2);

    let circuit = LiquidityCircuit {
        user_balance,
        liquidity_added,
        pool_total_liquidity,
        expected_new_total,
    };

    circuit.define(&mut api);

    let previous_proofs = vec![Proof { bytes: vec![0xAA; 32] }];
    crate::proof::generate_gkr_proof(&previous_proofs)
}

pub fn verify_liquidity_proof<C: Config<CircuitField = BN254>>(
    proof: &Proof,
    user_balance: U256,
    liquidity_added: U256,
    pool_total_liquidity: U256,
    expected_new_total: U256,
    previous_proofs: &[Proof],
) -> bool
where
    C::CircuitField: FieldForECC + PartialOrd + PrimeField,
{
    let (mut api, _input_vars, _public_vars) = RootBuilder::<BN254Config>::new(4, 2);
    let circuit = LiquidityCircuit {
        user_balance,
        liquidity_added,
        pool_total_liquidity,
        expected_new_total,
    };
    circuit.define(&mut api);

    if !verify_gkr_proof(proof, previous_proofs) {
        println!("❌ GKR Proof verification failed!");
        return false;
    }

    let pool_total_liquidity_var = api.constant(u256_to_bn254(pool_total_liquidity));
    let liquidity_added_var = api.constant(u256_to_bn254(liquidity_added));
    let expected_new_total_var = api.constant(u256_to_bn254(expected_new_total));

    let new_pool_total = api.add(pool_total_liquidity_var, liquidity_added_var);
    let diff = api.sub(new_pool_total, expected_new_total_var);
    let is_diff_zero = api.is_zero(diff);
    let one = api.constant(BN254::one());
    let is_not_zero = api.sub(one, is_diff_zero);
    if let Some(not_zero_val) = api.constant_value(is_not_zero) {
        if not_zero_val == BN254::one() {
            println!("❌ Pool total mismatch");
            return false;
        }
    }

    if liquidity_added > user_balance {
        println!("❌ Insufficient balance");
        return false;
    }

    true
}