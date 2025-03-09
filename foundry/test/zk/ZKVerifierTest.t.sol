// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {IZKVerifier} from "src/interfaces/IZKVerifier.sol";
import {DeployZKVerifier} from "script/DeployZkVerifier.s.sol";
import {PoseidonT3} from "poseidon-solidity/PoseidonT3.sol";

contract ZKVerifierTest is Test {
    IZKVerifier verifier;
    bytes[] previousProofs;
    int256[] publicOutputsAmount0;
    int256[] publicOutputsAmount1;
    address[] users;

    uint256 constant F = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

    function setUp() public {
        DeployZKVerifier deployer = new DeployZKVerifier();
        address verifierAddr = deployer.run();
        verifier = IZKVerifier(verifierAddr);

        string memory fileContent = vm.readFile("test/zk/proofs.txt");
        previousProofs = new bytes[](150);
        publicOutputsAmount0 = new int256[](150);
        publicOutputsAmount1 = new int256[](150);

        string[] memory lines = split(fileContent, "\n");
        uint256 proofCount = 0;
        uint256 outputCount = 0;
        for (uint256 i = 0; i < lines.length; i++) {
            string memory line = lines[i];
            if (bytes(line).length == 0) continue;
            if (startsWith(line, "Proof ") && proofCount < 150) {
                previousProofs[proofCount] = extractHex(line, "Proof ");
                proofCount++;
            } else if (startsWith(line, "Circuit ") && outputCount < 150) {
                bytes memory amount0Hex = extractHex(line, "Amount0: ");
                bytes memory amount1Hex = extractHex(line, "Amount1: ");
                console2.log("Circuit", outputCount, "Raw Amount0 Hex:", string(amount0Hex));
                console2.log("Circuit", outputCount, "Raw Amount1 Hex:", string(amount1Hex));

                bytes32 amount0Bytes32 = bytes32(amount0Hex);
                bytes32 amount1Bytes32 = bytes32(amount1Hex);
                // Extract the leftmost 20 bytes (160 bits) correctly
                uint256 amount0 = uint256(amount0Bytes32) >> 96; // Shift to align the 20 bytes
                uint256 amount1 = uint256(amount1Bytes32) >> 96; // Shift to align the 20 bytes

                console2.log("Circuit", outputCount, "Parsed Amount0 Uint:", amount0);
                console2.log("Circuit", outputCount, "Parsed Amount1 Uint:", amount1);

                // Verify against expected values for Circuit 0
                if (outputCount == 0) {
                    console2.logUint(6131000000000000000000);
                    console2.logUint(6350874878119819312338956282401532410528162663560392319966563075034087161855);
                }

                publicOutputsAmount0[outputCount] = int256(amount0);
                publicOutputsAmount1[outputCount] = int256(amount1);
                outputCount++;
            }
        }
        assertEq(proofCount, 150, "Incorrect number of proofs parsed");
        assertEq(outputCount, 150, "Incorrect number of public outputs parsed");

        users = new address[](10);
        for (uint256 i = 0; i < 10; i++) {
            users[i] = address(uint160(uint256(keccak256(abi.encodePacked("user", i)))));
            vm.deal(users[i], 1 ether);
        }
    }

    function testVerifyLiquidityProof() public {
        uint256 tasksPerUser = 15;
        uint256 totalTasks = users.length * tasksPerUser;
        assertEq(totalTasks, 150, "Total tasks should match proof count");

        for (uint256 userIdx = 0; userIdx < users.length; userIdx++) {
            address user = users[userIdx];
            vm.startBroadcast(user);

            uint256 startIdx = userIdx * tasksPerUser;
            uint256 endIdx = startIdx + tasksPerUser - 1;

            for (uint256 i = startIdx; i <= endIdx; i++) {
                bytes[] memory slicedProofs = slicePreviousProofs(i);
                uint256 amount0 =
                    uint256(publicOutputsAmount0[i] < 0 ? -publicOutputsAmount0[i] : publicOutputsAmount0[i]);
                uint256 amount1 =
                    uint256(publicOutputsAmount1[i] < 0 ? -publicOutputsAmount1[i] : publicOutputsAmount1[i]);

                console2.log("Proof", i, "length:", previousProofs[i].length);
                console2.log("Previous Proof[0] length:", slicedProofs[0].length);
                console2.log("Amount0:", amount0);
                console2.log("Amount1:", amount1);
                bool isValid = verifier.verifyLiquidityProof(previousProofs[i], slicedProofs, amount0, amount1);
                console2.log("User", userIdx);
                console2.log("Liquidity Proof", i);
                console2.log("isValid:", isValid);
                assertTrue(isValid, "Liquidity proof verification failed");
            }

            vm.stopBroadcast();
        }
    }

    function testVerifySwapProofFor() public {
        uint256 tasksPerUser = 15;
        uint256 totalTasks = users.length * tasksPerUser;
        assertEq(totalTasks, 150, "Total tasks should match proof count");

        for (uint256 userIdx = 0; userIdx < users.length; userIdx++) {
            address user = users[userIdx];
            vm.startBroadcast(user);

            uint256 startIdx = userIdx * tasksPerUser;
            uint256 endIdx = startIdx + tasksPerUser - 1;

            for (uint256 i = startIdx; i <= endIdx; i++) {
                bytes[] memory slicedProofs = slicePreviousProofs(i);
                uint256 amount0 =
                    uint256(publicOutputsAmount0[i] < 0 ? -publicOutputsAmount0[i] : publicOutputsAmount0[i]);

                uint256 amount1 =
                    uint256(publicOutputsAmount1[i] < 0 ? -publicOutputsAmount1[i] : publicOutputsAmount1[i]);

                console2.log("Using aggregated proof for index 0, length:", previousProofs[0].length);
                console2.log("Proof", i, "length:", previousProofs[i].length);
                console2.log("Previous Proof[0] length:", slicedProofs[0].length);
                console2.log("Amount0:", amount0);
                console2.log("Amount1:", amount1);
                bool isValid = verifier.verifySwapProof(previousProofs[i], slicedProofs, amount0, amount1);
                console2.log("User", userIdx);
                console2.log("Swap Proof", i);
                console2.log("isValid:", isValid);
                assertTrue(isValid, "Swap proof verification failed");
            }

            vm.stopBroadcast();
        }
    }

    function slicePreviousProofs(uint256 index) internal view returns (bytes[] memory) {
        bytes[] memory sliced = new bytes[](1);
        if (index == 0) {
            sliced[0] = hex"eb9cec8d1699b5c439fe68c321d9665a9f988d57b0f66630c484e9d7eafb3d08";
            console2.log("Using aggregated proof for index 0, length:", sliced[0].length);
        } else {
            sliced[0] = previousProofs[index - 1];
            console2.log("Using previous proof at index", index - 1, "length:", sliced[0].length);
        }
        return sliced;
    }

    function startsWith(string memory str, string memory prefix) internal pure returns (bool) {
        bytes memory strBytes = bytes(str);
        bytes memory prefixBytes = bytes(prefix);
        if (strBytes.length < prefixBytes.length) return false;
        for (uint256 i = 0; i < prefixBytes.length; i++) {
            if (strBytes[i] != prefixBytes[i]) return false;
        }
        return true;
    }

    function extractHex(string memory str, string memory prefix) internal pure returns (bytes memory) {
        bytes memory strBytes = bytes(str);
        uint256 start = 0;
        bytes memory prefixBytes = bytes(prefix);
        uint256 prefixStart = 0;
        for (uint256 i = 0; i <= strBytes.length - prefixBytes.length; i++) {
            bool isMatch = true;
            for (uint256 j = 0; j < prefixBytes.length; j++) {
                if (strBytes[i + j] != prefixBytes[j]) {
                    isMatch = false;
                    break;
                }
            }
            if (isMatch) {
                prefixStart = i;
                break;
            }
        }
        for (uint256 i = prefixStart + prefixBytes.length; i <= strBytes.length - 2; i++) {
            if (strBytes[i] == "0" && strBytes[i + 1] == "x") {
                start = i + 2;
                break;
            }
        }
        if (start == 0) {
            console2.log("Failed to find '0x' after prefix:", prefix);
            console2.log("In line:", str);
            revert("No '0x' found after prefix");
        }
        uint256 end = start;
        while (end < strBytes.length && strBytes[end] != "," && strBytes[end] != " " && strBytes[end] != "\n") {
            end++;
        }
        uint256 hexLength = end - start;
        if (hexLength != 64) {
            console2.log("Invalid hex length in:", str);
            console2.log("Prefix:", prefix);
            console2.log("Extracted length:", hexLength);
            console2.log("Extracted hex:", string(sliceBytes(strBytes, start, end)));
            revert("Invalid hex string length");
        }
        bytes memory hexBytes = sliceBytes(strBytes, start, end);
        return hexToBytes(string(hexBytes));
    }

    function split(string memory str, string memory delimiter) internal pure returns (string[] memory) {
        bytes memory strBytes = bytes(str);
        bytes memory delimBytes = bytes(delimiter);
        uint256 count = 1;
        for (uint256 i = 0; i < strBytes.length - delimBytes.length + 1; i++) {
            bool isMatch = true;
            for (uint256 j = 0; j < delimBytes.length; j++) {
                if (strBytes[i + j] != delimBytes[j]) {
                    isMatch = false;
                    break;
                }
            }
            if (isMatch) count++;
        }
        string[] memory result = new string[](count);
        uint256 lastIndex = 0;
        uint256 resultIndex = 0;
        for (uint256 i = 0; i < strBytes.length - delimBytes.length + 1; i++) {
            bool isMatch = true;
            for (uint256 j = 0; j < delimBytes.length; j++) {
                if (strBytes[i + j] != delimBytes[j]) {
                    isMatch = false;
                    break;
                }
            }
            if (isMatch) {
                result[resultIndex] = string(sliceBytes(strBytes, lastIndex, i));
                lastIndex = i + delimBytes.length;
                resultIndex++;
            }
        }
        result[resultIndex] = string(sliceBytes(strBytes, lastIndex, strBytes.length));
        return result;
    }

    function sliceBytes(bytes memory data, uint256 start, uint256 end) internal pure returns (bytes memory) {
        bytes memory result = new bytes(end - start);
        for (uint256 i = start; i < end; i++) {
            result[i - start] = data[i];
        }
        return result;
    }

    function hexToBytes(string memory hexStr) internal pure returns (bytes memory) {
        bytes memory hexBytes = bytes(hexStr);
        require(hexBytes.length % 2 == 0, "Invalid hex string length");
        bytes memory result = new bytes(hexBytes.length / 2);
        for (uint256 i = 0; i < hexBytes.length; i += 2) {
            result[i / 2] = bytes1(uint8(parseHexChar(hexBytes[i])) * 16 + uint8(parseHexChar(hexBytes[i + 1])));
        }
        return result;
    }

    function parseHexChar(bytes1 char) internal pure returns (uint8) {
        if (char >= "0" && char <= "9") return uint8(char) - uint8(bytes1("0"));
        if (char >= "a" && char <= "f") return uint8(char) - uint8(bytes1("a")) + 10;
        if (char >= "A" && char <= "F") return uint8(char) - uint8(bytes1("A")) + 10;
        revert("Invalid hex character");
    }

    function uintToHex(uint256 value) internal pure returns (string memory) {
        bytes memory alphabet = "0123456789abcdef";
        bytes memory str = new bytes(64);
        for (uint256 i = 0; i < 32; i++) {
            str[i * 2] = alphabet[uint8((value >> (4 * (31 - i))) & 0xf)];
            str[i * 2 + 1] = alphabet[uint8((value >> (4 * (31 - i) - 4)) & 0xf)];
        }
        return string(str);
    }
}
