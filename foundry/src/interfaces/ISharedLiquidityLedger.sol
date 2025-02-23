// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {IMerkleTree} from "./IMerkleTree.sol";
import {IZKVerifier} from "./IZKVerifier.sol";

uint256 constant TREE_DEPTH = 32;

interface ISharedLiquidityLedger is IMerkleTree, IZKVerifier {
    function updateLiquidityState(uint256 chainId, bytes32 newStateRoot, bytes memory zkProof) external;

    function getLatestLiquidityState(uint256 chainId) external view returns (bytes32);

    function getLatestLiquidityProof(uint256 chainId) external view returns (bytes memory);

    function insert(bytes32 leaf) external returns (bytes32 newRoot);

    function getMerkleProof(uint256 index) external view returns (bytes32[TREE_DEPTH] memory proof);

    function verifyProof(bytes32 leaf, bytes32[TREE_DEPTH] memory proof, bytes32 root, uint256 index)
        external
        view
        returns (bool);

    function getMerkleRoot() external view returns (bytes32);

    function stateTree() external view returns (IMerkleTree);

    function currentIndex() external view returns (uint256);

    function verifyLiquidityProof(
        uint256[2] memory proofA,
        uint256[2][2] memory proofB,
        uint256[2] memory proofC,
        uint256[4] memory publicSignals
    ) external view returns (bool);

    function verifySwapProof(
        uint256[2] memory proofA,
        uint256[2][2] memory proofB,
        uint256[2] memory proofC,
        uint256[5] memory publicSignals
    ) external view returns (bool);
}
