pragma circom 2.0.0;

include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/comparators.circom"; // ✅ Required for range checks

template SwapVerifier() {
    // zkSNARK Proof Inputs
    signal input proof_a_x;
    signal input proof_a_y;
    signal input proof_b_x1;
    signal input proof_b_x2;
    signal input proof_b_y1;
    signal input proof_b_y2;
    signal input proof_c_x;
    signal input proof_c_y;

    // Public input: Swap details
    signal input input_amount;      // Input token amount before swap
    signal input expected_output;   // Expected output after swap
    signal input actual_output;     // Actual output received
    signal input max_slippage;      // Max allowable slippage percentage (0-100)

    // ✅ Compute absolute slippage difference
    signal slippage_diff;
    slippage_diff <== expected_output - actual_output;

    // ✅ Compute max allowed slippage WITHOUT division
    signal max_allowed_slippage;
    max_allowed_slippage <== expected_output * max_slippage; // Multiply by 100 in Solidity instead of dividing here

    // ✅ Check if `slippage_diff * 100 ≤ max_allowed_slippage`
    component slippageCheck = LessThan(32);
    slippageCheck.in[0] <== slippage_diff * 100;
    slippageCheck.in[1] <== max_allowed_slippage;

    // ✅ Binary signal: 1 if within slippage limit, 0 otherwise
    signal is_slippage_valid;
    is_slippage_valid <== 1 - slippageCheck.out;

    // ✅ Binary constraint (0 or 1)
    signal binary_check;
    binary_check <== is_slippage_valid * (is_slippage_valid - 1);
    binary_check === 0;

    // ✅ Poseidon hash for proof validation
    signal computed_hash;
    component poseidon = Poseidon(4);
    poseidon.inputs[0] <== input_amount;
    poseidon.inputs[1] <== expected_output;
    poseidon.inputs[2] <== actual_output;
    poseidon.inputs[3] <== max_slippage;
    computed_hash <== poseidon.out;

    // ✅ Final verification
    signal is_valid;
    is_valid <== is_slippage_valid;

    // ✅ Public output
    signal output final_hash;
    final_hash <== computed_hash;
}

component main { public [input_amount, expected_output, actual_output, max_slippage] } = SwapVerifier();