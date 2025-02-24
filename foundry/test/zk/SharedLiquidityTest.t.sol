// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {SharedLiquidityLedger} from "src/zk/SharedLiquidityLedger.sol";
import {DeploySharedLedger} from "script/DeploySharedLedger.s.sol";
import {MerkleTree} from "src/zk/MerkleTree.sol";
import {ZKVerifier} from "src/zk/ZKVerifier.sol";

contract SharedLiquidityTest is Test {
    SharedLiquidityLedger ledger;
    MerkleTree stateTree;
    ZKVerifier zkVerifier;

    function setUp() public {
        DeploySharedLedger deployer = new DeploySharedLedger();
        address ledgerAddr = deployer.run(); // No broadcast, avoids error in Foundry tests
        ledger = SharedLiquidityLedger(ledgerAddr);

        // zkVerifier = ledger.zkVerifier();
        // stateTree = ledger.stateTree();
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
        }
        return result;
    }

    // function testUpdateLiquidityState() public {
    //     uint256 chainId = 1; // Example chain ID
    //     bytes32 newStateRoot = keccak256(abi.encodePacked(block.timestamp)); // Simulate new root

    //     // Load proof JSON
    //     string memory proofJson = _loadJson("test/liquidityverifier_proof.json");
    //     string memory publicJson = _loadJson("test/liquidityverifier_public.json");

    //     console2.log("Proof JSON:", proofJson);
    //     console2.log("Public JSON:", publicJson);

    //     // Decode JSON
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

    //     // Convert public signals to uint256[4]
    //     uint256[4] memory publicSignals = [
    //         _parseUint256(pubSignalsStr[0]),
    //         _parseUint256(pubSignalsStr[1]),
    //         _parseUint256(pubSignalsStr[2]),
    //         _parseUint256(pubSignalsStr[3])
    //     ];

    //     bytes memory zkProof = abi.encode(proofA, proofB, proofC, publicSignals);

    //     // Verify liquidity state update
    //     ledger.updateLiquidityState(chainId, newStateRoot, zkProof);

    //     // Retrieve latest state root
    //     bytes32 retrievedRoot = ledger.getLatestLiquidityState(chainId);
    //     assertEq(retrievedRoot, newStateRoot, "State root mismatch");

    //     console2.log("Updated State Root:");
    //     console2.logBytes32(retrievedRoot);
    // }
}
