// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

uint256 constant TREE_DEPTH = 32;

/// @title Interface for MerkleTree

interface IMerkleTree {
    function insert(bytes32 leaf) external returns (bytes32 newRoot);

    function getMerkleProof(uint256 index) external view returns (bytes32[TREE_DEPTH] memory proof);

    function verifyProof(bytes32 leaf, bytes32[TREE_DEPTH] memory proof, bytes32 root, uint256 index)
        external
        view
        returns (bool);

    function getMerkleRoot() external view returns (bytes32);
}
