// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";
import {console2} from "forge-std/Console2.sol";

contract MerkleTree {
    using PoseidonHasherLibrary for bytes32[];

    uint256 public constant TREE_DEPTH = 32;
    bytes32[TREE_DEPTH] public zeroes; // Default zero hashes for an empty tree
    bytes32[TREE_DEPTH] public filledSubtrees; // Stores intermediate hashes
    uint256 public currentIndex; // Next available index for a new leaf
    bytes32 public merkleRoot; // Current Merkle root

    event LeafInserted(uint256 indexed index, bytes32 leaf, bytes32 newRoot);

    constructor() {
        // Initialize zeroes
        zeroes[0] = bytes32(0);
        for (uint256 i = 1; i < TREE_DEPTH; i++) {
            zeroes[i] = PoseidonHasherLibrary.hashSingle(zeroes[i - 1], zeroes[i - 1]);
        }

        // Initialize filled subtrees with zero values
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            filledSubtrees[i] = zeroes[i];
        }

        merkleRoot = zeroes[TREE_DEPTH - 1]; // Start with an empty tree root
    }

    /// @notice Inserts a new leaf into the Merkle tree and updates the root
    function insert(bytes32 leaf) external returns (bytes32 newRoot) {
        require(currentIndex < 2 ** TREE_DEPTH, "MerkleTree: tree is full");

        uint256 index = currentIndex;
        currentIndex++;

        bytes32 node = leaf;
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            // console2.log("Level", i, "Index:", index);
            // console2.log("Node:");
            // console2.logBytes32(node);

            if ((index & 1) == 0) {
                filledSubtrees[i] = node;
                node = PoseidonHasherLibrary.hashSingle(node, zeroes[i]);
            } else {
                node = PoseidonHasherLibrary.hashSingle(filledSubtrees[i], node);
            }
            index >>= 1;
        }

        merkleRoot = node;
        // console2.log("Updated Merkle Root:");
        // console2.logBytes32(merkleRoot);
        emit LeafInserted(currentIndex - 1, leaf, node);
        return merkleRoot;
    }

    /// @notice Computes a Merkle proof for a given leaf
    function getMerkleProof(uint256 index) external view returns (bytes32[TREE_DEPTH] memory proof) {
        require(index < currentIndex, "MerkleTree: index out of bounds");
        uint256 currentIdx = index;
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            if ((currentIdx & 1) == 0) {
                // Left child, sibling is zero (right not inserted)
                proof[i] = zeroes[i];
            } else {
                // Right child, sibling is filledSubtrees[i] (left)
                proof[i] = filledSubtrees[i];
            }
            currentIdx >>= 1;
        }
        return proof;
    }

    /// @notice Verifies a Merkle proof
    function verifyProof(bytes32 leaf, bytes32[TREE_DEPTH] memory proof, bytes32 root, uint256 index)
        external
        view
        returns (bool)
    {
        bytes32 node = leaf;
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            if ((index >> i) & 1 == 0) {
                node = PoseidonHasherLibrary.hashSingle(node, proof[i]);
            } else {
                node = PoseidonHasherLibrary.hashSingle(proof[i], node);
            }
        }
        return node == root;
    }

    function getMerkleRoot() external view returns (bytes32) {
        return merkleRoot;
    }
}
