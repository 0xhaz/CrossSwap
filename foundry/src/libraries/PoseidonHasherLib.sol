// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {PoseidonHasher} from "src/libraries/PoseidonHasher.sol";
import {console2} from "forge-std/Console2.sol";

library PoseidonHasherLibrary {
    uint256 constant F = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001;

    function hashSingle(bytes32 left, bytes32 right) internal pure returns (bytes32) {
        uint256[2] memory inputs = [uint256(left), uint256(right)];
        bytes32 hash = bytes32(PoseidonHasher.hash(inputs));
        return hash;
    }

    function reverseBytes(bytes32 input) internal pure returns (bytes32) {
        uint256 x = uint256(input);
        uint256 output;
        for (uint256 i = 0; i < 32; i++) {
            output |= ((x >> (i * 8)) & 0xFF) << ((31 - i) * 8);
        }
        return bytes32(output);
    }
}
