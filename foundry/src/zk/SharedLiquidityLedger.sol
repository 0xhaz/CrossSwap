// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {MerkleTree} from "src/zk/MerkleTree.sol";
import {PoseidonHasher} from "src/libraries/PoseidonHasher.sol";

contract SharedLiquidityLedger {
    MerkleTree public stateTree;
    ZKVerifier public zkVerifier;

    uint256 public constant TREE_DEPTH = 32;

    mapping(uint256 => bytes32) public liquidityStates; // Mapping of chainId to stateRoot
    mapping(uint256 => bytes) public liquidityProofs; // Mapping of chainId to proof

    event LiquidityStateUpdated(uint256 indexed chainId, bytes32 stateRoot, bytes proof);

    constructor(address _zkVerifier) {
        zkVerifier = ZKVerifier(_zkVerifier);
        stateTree = new MerkleTree();
    }

    /// @notice Updates the liqudity state tree with a new state root
    /// @param chainId The chain ID
    /// @param newStateRoot The new state root
    /// @param proof The proof for the new state root
    function updateLiquidityState(uint256 chainId, bytes32 newStateRoot, bytes memory proof) external {
        require(zkVerifier.verifyProof(proof), "SharedLiquidityLedger: invalid proof");

        // Get the correct Merkle Proof using the last inserted index
        uint256 latestIndex = stateTree.currentIndex() - 1;
        bytes32[TREE_DEPTH] memory merkleProof = stateTree.getMerkleProof(latestIndex);

        require(
            stateTree.verifyProof(newStateRoot, merkleProof, stateTree.getMerkleRoot()),
            "SharedLiquidityLedger: invalid state root"
        );

        liquidityStates[chainId] = newStateRoot;
        liquidityProofs[chainId] = proof;
        stateTree.insert(newStateRoot);

        emit LiquidityStateUpdated(chainId, newStateRoot, proof);
    }

    /// @notice Fetches the latest state root for a chain
    function getLatestLiquidityState(uint256 chainId) external view returns (bytes32) {
        return liquidityStates[chainId];
    }

    /// @notice Fetches the proof for the latest state root for a chain
    function getLatestLiquidityProof(uint256 chainId) external view returns (bytes memory) {
        return liquidityProofs[chainId];
    }
}
