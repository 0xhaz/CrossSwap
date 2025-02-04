// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";

contract PoseidonHasherTest is Test {
    function testPoseidonHash() public pure {
        bytes32 left = bytes32(uint256(123456));
        bytes32 right = bytes32(uint256(7891011));

        bytes32 actualHash = PoseidonHasherLibrary.hashSingle(left, right);

        console2.logBytes32(actualHash);
    }
}
