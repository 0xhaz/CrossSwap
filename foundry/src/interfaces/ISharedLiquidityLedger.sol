// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {IMerkleTree} from "./IMerkleTree.sol";
import {IZKVerifier} from "./IZKVerifier.sol";

uint256 constant TREE_DEPTH = 20;

interface ISharedLiquidityLedger is IMerkleTree, IZKVerifier {
    function updateLiquidityState(
        uint256 chainId,
        bytes32 newStateRoot,
        bytes memory zkProof,
        bytes[] memory previousProofs,
        int256 amount0,
        int256 amount1
    ) external;

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
}
