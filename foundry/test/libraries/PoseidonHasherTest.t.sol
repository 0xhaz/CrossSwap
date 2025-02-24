// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {PoseidonHasherLibrary, PoseidonHasher} from "src/libraries/PoseidonHasherLib.sol";

contract PoseidonHasherTest is Test {
    using PoseidonHasherLibrary for bytes32;

    function testHashSingle() public pure {
        bytes32 left = keccak256("test-left");
        bytes32 right = keccak256("test-right");

        uint256[2] memory inputs = [uint256(left), uint256(right)];
        bytes32 expectedHash = bytes32(PoseidonHasher.hash(inputs));

        bytes32 computedHash = PoseidonHasherLibrary.hashSingle(left, right);

        assertEq(computedHash, expectedHash);
    }

    function testPoseidonBigEndian() public pure {
        bytes32 left = keccak256("test-left");
        bytes32 right = keccak256("test-right");

        uint256[2] memory inputs = [uint256(left), uint256(right)];
        bytes32 bigEndianHash = bytes32(PoseidonHasher.hash(inputs));

        // console2.log("Big-Endian Poseidon Hash:");
        // console2.logBytes32(bigEndianHash);

        assertEq(bigEndianHash, PoseidonHasherLibrary.hashSingle(left, right));
    }

    function testPoseidonLittleEndian() public pure {
        bytes32 left = keccak256("test-left");
        bytes32 right = keccak256("test-right");

        bytes32 leftLE = PoseidonHasherLibrary.reverseBytes(left);
        bytes32 rightLE = PoseidonHasherLibrary.reverseBytes(right);

        uint256[2] memory inputs = [uint256(leftLE), uint256(rightLE)];
        bytes32 littleEndianHash = bytes32(PoseidonHasher.hash(inputs));

        // console2.log("Little-Endian Poseidon Hash:");
        // console2.logBytes32(littleEndianHash);
    }

    function testReverseBytes() public pure {
        bytes32 input = hex"0102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F20";
        bytes32 expectedReversed = hex"201F1E1D1C1B1A191817161514131211100F0E0D0C0B0A090807060504030201";

        bytes32 reversed = PoseidonHasherLibrary.reverseBytes(input);
        assertEq(reversed, expectedReversed);
    }

    function testHashDeterministic() public pure {
        bytes32 left = keccak256("test-left");
        bytes32 right = keccak256("test-right");

        bytes32 hash1 = PoseidonHasherLibrary.hashSingle(left, right);
        bytes32 hash2 = PoseidonHasherLibrary.hashSingle(left, right);

        assertEq(hash1, hash2);
    }

    function testHashDifferentInputs() public pure {
        bytes32 left = keccak256("input-1");
        bytes32 right = keccak256("input-2");
        bytes32 differentRight = keccak256("input-3");

        bytes32 hash1 = PoseidonHasherLibrary.hashSingle(left, right);
        bytes32 hash2 = PoseidonHasherLibrary.hashSingle(left, differentRight);

        assertFalse(hash1 == hash2);
    }
}
