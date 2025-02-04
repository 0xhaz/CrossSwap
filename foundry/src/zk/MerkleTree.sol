// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";

contract MerkleTree {
    using PoseidonHasherLibrary for bytes32[];

    uint256 public constant TREE_DEPTH = 32;
    bytes32[TREE_DEPTH] public zeroes; // Default zero hashes for an empty tree
    bytes32[TREE_DEPTH] public filledSubtrees; // Stores intermediate hashes
    uint256 public currentIndex; // Next available index for a new leaf
    bytes32 public merkleRoot; // Current Merkle root

    event LeafInserted(uint256 indexed index, bytes32 leaf, bytes32 newRoot);

    constructor() {
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            zeroes[i] = PoseidonHasherLibrary.hashSingle(bytes32(i), bytes32(i)); // Generate a default zero hash
            if (i == 0) {
                filledSubtrees[i] = zeroes[i];
            } else {
                filledSubtrees[i] = PoseidonHasherLibrary.hashSingle(filledSubtrees[i - 1], zeroes[i]);
            }
        }

        merkleRoot = filledSubtrees[TREE_DEPTH - 1];
    }

    /// @notice Inserts a new leaf into the Merkle tree and updates the root
    function insert(bytes32 leaf) external returns (bytes32 newRoot) {
        require(currentIndex < 2 ** TREE_DEPTH, "MerkleTree: tree is full");

        uint256 index = currentIndex;
        currentIndex++;

        bytes32 node = leaf;
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            if ((index & 1) == 0) {
                filledSubtrees[i] = node;
                node = PoseidonHasherLibrary.hashSingle(node, zeroes[i]);
            } else {
                node = PoseidonHasherLibrary.hashSingle(filledSubtrees[i], node);
            }
            index >>= 1;
        }

        merkleRoot = node;
        emit LeafInserted(currentIndex - 1, leaf, node);
        return merkleRoot;
    }

    function getMerkleRoot() external view returns (bytes32) {
        return merkleRoot;
    }

    /// @notice Computes a Merkle proof for a given leaf
    function getMerkleProof(uint256 index) external view returns (bytes32[TREE_DEPTH] memory proof) {
        require(index < currentIndex, "MerkleTree: index out of bounds");

        bytes32 node = zeroes[0];
        uint256 currentIdx = index;

        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            proof[i] = (currentIdx & 1) == 0 ? filledSubtrees[i] : zeroes[i];
            currentIdx >>= 1;
        }
        return proof;
    }

    /// @notice Verifies a Merkle proof
    function verifyProof(bytes32 leaf, bytes32[TREE_DEPTH] memory proof, bytes32 root) external pure returns (bool) {
        bytes32 node = leaf;
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            node = PoseidonHasherLibrary.hashSingle(node, proof[i]);
        }
        return node == root;
    }
}
