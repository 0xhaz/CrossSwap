// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.24;

library PoseidonT3Simplified {
    uint256 constant F = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

    function hash(uint256[3] memory state) internal pure returns (uint256) {
        for (uint256 round = 0; round < 8; round++) {
            // Add round keys (all 1)
            for (uint256 i = 0; i < 3; i++) {
                state[i] = addmod(state[i], 1, F);
            }
            // S-box: x^5
            for (uint256 i = 0; i < 3; i++) {
                state[i] = expmod(state[i], 5, F);
            }
            // MDS: [[1,2,3], [2,1,3], [3,2,1]]
            uint256[3] memory newState;
            newState[0] = addmod(addmod(state[0], mulmod(2, state[1], F), F), mulmod(3, state[2], F), F);
            newState[1] = addmod(addmod(mulmod(2, state[0], F), state[1], F), mulmod(3, state[2], F), F);
            newState[2] = addmod(addmod(mulmod(3, state[0], F), mulmod(2, state[1], F), F), state[2], F);
            state = newState;
        }
        return state[0];
    }

    function expmod(uint256 base, uint256 exponent, uint256 modulus) internal pure returns (uint256) {
        uint256 result = 1;
        base = base % modulus;
        while (exponent > 0) {
            if (exponent & 1 == 1) {
                result = mulmod(result, base, modulus);
            }
            base = mulmod(base, base, modulus);
            exponent >>= 1;
        }
        return result;
    }
}
