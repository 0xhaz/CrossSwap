pragma circom  2.0.0;

include "node_modules/circomlib/circuits/poseidon.circom";

template LiquidityVerifier() {
    // zkSNARK Proof Inputs
    signal input proof_a_x;
    signal input proof_a_y;
    signal input proof_b_x1;
    signal input proof_b_x2;
    signal input proof_b_y1;
    signal input proof_b_y2;
    signal input proof_c_x;
    signal input proof_c_y;    

    // Public input: Liquidity details
    signal input deposited_amount; // The amount token deposited on Chain A
    signal input received_amount; // The amount token received on Chain B
    signal input user_address; // The user's address

    // Hash public input using Poseidon
    signal computed_hash;
    component poseidon = Poseidon(3); // Hash 3 inputs (deposited_amount, received_amount, user_address)
    poseidon.inputs[0] <== deposited_amount;
    poseidon.inputs[1] <== received_amount;
    poseidon.inputs[2] <== user_address;
    computed_hash <== poseidon.out;

    // Ensure deposited amount == received amount
    // This guarantees liquidity integrity across chains
    deposited_amount === received_amount;

    // Store the computed hash as the public output
    signal output final_hash;
    final_hash <== computed_hash;

}

component main { public [deposited_amount, received_amount, user_address] } = LiquidityVerifier();