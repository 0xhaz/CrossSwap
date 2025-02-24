// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {MerkleTree} from "src/zk/MerkleTree.sol";
import {IZKVerifier} from "src/interfaces/IZKVerifier.sol";
import {PoseidonHasher} from "src/libraries/PoseidonHasher.sol";

contract SharedLiquidityLedger {
    MerkleTree public stateTree;
    IZKVerifier public zkVerifier;

    uint256 public constant TREE_DEPTH = 32;

    mapping(uint256 => bytes32) public liquidityStates; // Mapping of chainId to stateRoot
    mapping(uint256 => bytes) public zkProofs; // Mapping of chainId to proof

    event LiquidityStateUpdated(uint256 indexed chainId, bytes32 stateRoot, bytes proof);

    constructor(address _zkVerifier) {
        zkVerifier = IZKVerifier(_zkVerifier);
        stateTree = new MerkleTree();
    }

    /// @notice Updates the liquidity state tree with a new state root
    /// @param chainId The chain ID
    /// @param newStateRoot The new state root
    /// @param zkProof The zk-SNARK proof
    function updateLiquidityState(uint256 chainId, bytes32 newStateRoot, bytes memory zkProof) external {
        (
            uint256[2] memory proofA,
            uint256[2][2] memory proofB,
            uint256[2] memory proofC,
            uint256[4] memory publicSignals
        ) = _decodeZkProof(zkProof);

        // Verify zk-SNARK proof (Ensure function expects uint256[3])
        require(
            zkVerifier.verifyLiquidityProof(proofA, proofB, proofC, publicSignals),
            "SharedLiquidityLedger: invalid zkSNARK proof"
        );

        _validateStateRoot(newStateRoot);
        stateTree.insert(newStateRoot);

        liquidityStates[chainId] = newStateRoot;
        zkProofs[chainId] = zkProof;

        emit LiquidityStateUpdated(chainId, newStateRoot, zkProof);
    }

    /// @notice Fetches the latest state root for a chain
    function getLatestLiquidityState(uint256 chainId) external view returns (bytes32) {
        return liquidityStates[chainId];
    }

    /// @notice Fetches the proof for the latest state root for a chain
    function getLatestLiquidityProof(uint256 chainId) external view returns (bytes memory) {
        return zkProofs[chainId];
    }

    /// @dev Decodes zk-SNARK proof from bytes
    function _decodeZkProof(bytes memory zkProof)
        internal
        pure
        returns (
            uint256[2] memory proofA,
            uint256[2][2] memory proofB,
            uint256[2] memory proofC,
            uint256[4] memory publicSignals // Ensure correct size
        )
    {
        return abi.decode(zkProof, (uint256[2], uint256[2][2], uint256[2], uint256[4])); // Ensure it decodes 3 elements
    }

    /// @dev Ensures that the new state root is valid
    function _validateStateRoot(bytes32 newStateRoot) internal view {
        bytes32 latestStateRoot = stateTree.getMerkleRoot();
        require(newStateRoot != latestStateRoot, "SharedLiquidityLedger: state root unchanged");
        require(newStateRoot != bytes32(0), "SharedLiquidityLedger: invalid state root");
    }
}
