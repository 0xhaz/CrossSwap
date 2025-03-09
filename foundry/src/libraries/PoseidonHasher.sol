// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/// @title Poseidon Hash Implementation in Solidity
/// @notice Implements a Poseidon hash compatible with Rust Poseidon::new(8, 1, 2)
library PoseidonHasher {
    uint256 internal constant F = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001;
    uint256 internal constant ROUNDS = 8;

    /// @dev Simplified MDS matrix (3x3) matching Rust: [[1, 2, 3], [2, 1, 3], [3, 2, 1]]
    function getMdsMatrix() internal pure returns (uint256[3][3] memory) {
        return [
            [uint256(1), uint256(2), uint256(3)],
            [uint256(2), uint256(1), uint256(3)],
            [uint256(3), uint256(2), uint256(1)]
        ];
    }

    /// @dev Simplified round constants (8 * 3 = 24 constants, demo uses 1)
    function getRoundConstants() internal pure returns (uint256[24] memory) {
        uint256[24] memory constants;
        for (uint256 i = 0; i < 24; i++) {
            constants[i] = 1; // Simplified; use precomputed values in production
        }
        return constants;
    }

    /// @dev Poseidon hash for 2 inputs (rate 2)
    function hash(uint256[2] memory inputs) internal pure returns (uint256) {
        uint256[24] memory C = getRoundConstants();
        uint256[3][3] memory M = getMdsMatrix();

        // State: [input0, input1, capacity=0]
        uint256[3] memory state = [inputs[0], inputs[1], uint256(0)];

        for (uint256 r = 0; r < ROUNDS; r++) {
            // Add round constants
            for (uint256 i = 0; i < 3; i++) {
                state[i] = addmod(state[i], C[r * 3 + i], F);
            }

            // S-box: x^5
            for (uint256 i = 0; i < 3; i++) {
                state[i] = pow5(state[i]);
            }

            // Mix layer (MDS multiplication)
            uint256[3] memory newState;
            for (uint256 i = 0; i < 3; i++) {
                newState[i] = 0;
                for (uint256 j = 0; j < 3; j++) {
                    newState[i] = addmod(newState[i], mulmod(state[j], M[i][j], F), F);
                }
            }
            state = newState;
        }

        return state[0]; // Output first element
    }

    /// @dev Computes n^5 mod F
    function pow5(uint256 n) internal pure returns (uint256) {
        uint256 pow2 = mulmod(n, n, F);
        uint256 pow4 = mulmod(pow2, pow2, F);
        return mulmod(n, pow4, F);
    }
}
