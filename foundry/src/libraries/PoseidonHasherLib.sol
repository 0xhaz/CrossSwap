// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {console2} from "forge-std/Console2.sol";
import {PoseidonHasher} from "src/libraries/PoseidonHasher.sol";

/// @title Poseidon Hasher Library
/// @notice Provides Poseidon hash functions optimized for zkSNARKs
library PoseidonHasherLibrary {
    uint256 constant F = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001;

    /// @notice Poseidon hash function for two inputs
    function hashSingle(bytes32 left, bytes32 right) internal pure returns (bytes32) {
        uint256[2] memory inputs = [uint256(left), uint256(right)];
        return bytes32(PoseidonHasher.hash(inputs));
    }

    /// @notice Hashes an array of values using iterative Poseidon hashing
    function hashMultiple(bytes32[] memory values) internal pure returns (bytes32) {
        require(values.length > 0, "PoseidonHasher: values length must be greater than 0");

        bytes32 rollingHash = values[0];
        for (uint256 i = 1; i < values.length; i++) {
            rollingHash = hashSingle(rollingHash, values[i]);
        }

        return rollingHash;
    }
}
