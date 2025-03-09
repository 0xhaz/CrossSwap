// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {MerkleTree} from "src/zk/MerkleTree.sol";
import {DeployMerkleTree} from "script/DeployMerkleTree.s.sol";
import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";

contract MerkleTreeTest is Test {
    MerkleTree tree;
    uint256 public constant TREE_DEPTH = 20;

    function setUp() public {
        DeployMerkleTree deployer = new DeployMerkleTree();
        address treeAddr = deployer.run();
        tree = MerkleTree(treeAddr);
    }

    function testMerkleTreeInitialization() public view {
        bytes32 expectedRoot = tree.getMerkleRoot();
        assertEq(expectedRoot, tree.getMerkleRoot());
    }

    function testInsertLeaf() public {
        bytes32 leaf = keccak256("leaf-1");

        bytes32 oldRoot = tree.getMerkleRoot();
        bytes32 newRoot = tree.insert(leaf);

        assertNotEq(newRoot, oldRoot);
        assertEq(newRoot, tree.getMerkleRoot());
    }

    function testInsertMultipleLeaves() public {
        bytes32 leaf1 = keccak256("leaf-1");
        bytes32 leaf2 = keccak256("leaf-2");

        bytes32 root1 = tree.insert(leaf1);
        bytes32 root2 = tree.insert(leaf2);

        assertNotEq(root1, root2);
        assertEq(root2, tree.getMerkleRoot());
    }

    function testMerkleProofVerification() public {
        bytes32 leaf = keccak256("leaf-1");
        uint256 leafIndex = 0; // Specify the index explicitly
        bytes32 insertedRoot = tree.insert(leaf);

        bytes32[TREE_DEPTH] memory proof = tree.getMerkleProof(leafIndex);
        bytes32 computedRoot = tree.getMerkleRoot();

        // console2.log("Inserted Root:");
        // console2.logBytes32(insertedRoot);
        // console2.log("Computed Root:");
        // console2.logBytes32(computedRoot);

        bool isValid = tree.verifyProof(leaf, proof, computedRoot, leafIndex); // Pass leafIndex
        assertTrue(isValid, "Merkle proof verification failed");
    }

    function testHashFunction() public pure {
        bytes32 left = keccak256("left");
        bytes32 right = keccak256("right");
        bytes32 poseidonHash = PoseidonHasherLibrary.hashSingle(left, right);

        // console2.log("Left:");
        // console2.logBytes32(left);
        // console2.log("Right:");
        // console2.logBytes32(right);
        // console2.log("Poseidon Hash:");
        // console2.logBytes32(poseidonHash);

        assertTrue(poseidonHash != bytes32(0), "Hash output should not be zero");
    }

    function testMerkleTreeInsertion() public {
        bytes32 leaf = keccak256("testLeaf");
        bytes32 expectedRoot = tree.insert(leaf);

        // console2.log("Inserted Leaf:");
        // console2.logBytes32(leaf);
        // console2.log("Updated Root:");
        // console2.logBytes32(expectedRoot);

        assertEq(tree.getMerkleRoot(), expectedRoot, "Merkle root mismatch after insertion");
    }

    function testMerkleProofAndVerification() public {
        bytes32 leaf = keccak256("testLeaf");
        uint256 leafIndex = 0; // Inserting at index 0
        tree.insert(leaf);

        // Generate Merkle proof
        bytes32[20] memory proof = tree.getMerkleProof(leafIndex);
        bytes32 root = tree.getMerkleRoot();

        // console2.log("Generated Proof:");
        // for (uint256 i = 0; i < proof.length; i++) {
        //     console2.logBytes32(proof[i]);
        // }

        // console2.log("Stored Merkle Root:");
        // console2.logBytes32(root);

        // Verify proof with leaf index
        bool isValid = tree.verifyProof(leaf, proof, root, leafIndex);
        assertTrue(isValid, "Merkle proof verification failed");
    }

    function testInsertMultipleAndVerify() public {
        bytes32 leaf1 = keccak256("leaf-1");
        bytes32 leaf2 = keccak256("leaf-2");

        uint256 index1 = 0; // First leaf index
        uint256 index2 = 1; // Second leaf index

        bytes32 root1 = tree.insert(leaf1);
        bytes32 root2 = tree.insert(leaf2);

        // console2.log("Root after first insert:");
        // console2.logBytes32(root1);
        // console2.log("Root after second insert:");
        // console2.logBytes32(root2);

        // Get proofs for each inserted leaf
        bytes32[TREE_DEPTH] memory proof1 = tree.getMerkleProof(index1);
        bool isValid1 = tree.verifyProof(leaf1, proof1, root1, index1); // Pass index
        assertTrue(isValid1, "Proof verification failed for leaf1");

        bytes32[TREE_DEPTH] memory proof2 = tree.getMerkleProof(index2);
        bool isValid2 = tree.verifyProof(leaf2, proof2, root2, index2); // Pass index
        assertTrue(isValid2, "Proof verification failed for leaf2");
    }

    function testMerkleProofValidation() public {
        bytes32 leaf = keccak256("testLeaf");
        uint256 index = 0; // Leaf index in the tree

        tree.insert(leaf);

        // Generate proof for the inserted leaf
        bytes32[TREE_DEPTH] memory proof = tree.getMerkleProof(index);
        bytes32 computedRoot = tree.getMerkleRoot();

        // console2.log("Stored Merkle Root:");
        // console2.logBytes32(computedRoot);
        // console2.log("Generated Proof Nodes:");
        // for (uint256 i = 0; i < proof.length; i++) {
        //     console2.logBytes32(proof[i]);
        // }

        // Verify the proof (passing the index)
        bool isValid = tree.verifyProof(leaf, proof, computedRoot, index);
        assertTrue(isValid, "Merkle proof verification failed");
    }
}
