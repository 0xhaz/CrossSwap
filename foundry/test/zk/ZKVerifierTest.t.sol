// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {DeployZKVerifier} from "script/DeployZkVerifier.s.sol";
// import {SwapVerifier} from "src/zk/SwapVerifier.sol";

contract ZKVerifierTest is Test {
    ZKVerifier verifier;
    // SwapVerifier verifier;

    function setUp() public {
        DeployZKVerifier deployer = new DeployZKVerifier();
        address verifierAddr = deployer.run();
        verifier = ZKVerifier(verifierAddr);
        // verifier = SwapVerifier(verifierAddr);
    }

    /// @notice Reads JSON file using Foundry's `ffi()`
    function _loadJson(string memory filePath) internal returns (string memory) {
        string[] memory command = new string[](2);
        command[0] = "cat";
        command[1] = filePath;
        bytes memory res = vm.ffi(command);
        return string(res);
    }

    /// @notice Converts a numeric string to `uint256`
    function _parseUint256(string memory str) internal pure returns (uint256) {
        bytes memory b = bytes(str);
        uint256 result = 0;
        for (uint256 i = 0; i < b.length; i++) {
            require(b[i] >= 0x30 && b[i] <= 0x39, "Invalid character in uint256 string");
            result = result * 10 + (uint256(uint8(b[i])) - 48);
            //
            return result;
        }

        // function testVerifySwapProof() public {
        //     // Load proof JSON
        //     string memory proofJson = _loadJson("test/swapverifier_proof.json");
        //     string memory publicJson = _loadJson("test/swapverifier_public.json");

        //     console2.log("Proof JSON:", proofJson);
        //     console2.log("Public JSON:", publicJson);

        //     // Decode JSON strings into Solidity-friendly format
        //     (string[] memory pi_a, string[][] memory pi_b, string[] memory pi_c) =
        //         abi.decode(bytes(proofJson), (string[], string[][], string[]));

        //     string[] memory pubSignalsStr = abi.decode(bytes(publicJson), (string[]));

        //     // Convert proof values to uint256
        //     uint256[2] memory proofA = [_parseUint256(pi_a[0]), _parseUint256(pi_a[1])];

        //     uint256[2][2] memory proofB = [
        //         [_parseUint256(pi_b[0][0]), _parseUint256(pi_b[0][1])],
        //         [_parseUint256(pi_b[1][0]), _parseUint256(pi_b[1][1])]
        //     ];

        //     uint256[2] memory proofC = [_parseUint256(pi_c[0]), _parseUint256(pi_c[1])];

        //     // Convert public signals to uint256 array
        //     uint256[5] memory publicSignals = [
        //         _parseUint256(pubSignalsStr[0]),
        //         _parseUint256(pubSignalsStr[1]),
        //         _parseUint256(pubSignalsStr[2]),
        //         _parseUint256(pubSignalsStr[3]),
        //         _parseUint256(pubSignalsStr[4])
        //     ];

        //     // Call verification function
        //     bool isValid = verifier.verifySwapProof(proofA, proofB, proofC, publicSignals);
        //     assertTrue(isValid, "ZK Proof verification failed");
        // }
    }
}
