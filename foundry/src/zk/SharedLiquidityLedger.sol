// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {MerkleTree} from "src/zk/MerkleTree.sol";

import {IZKVerifier} from "src/interfaces/IZKVerifier.sol";
import {PoseidonHasher} from "src/libraries/PoseidonHasher.sol";
import {IMerkleTree} from "src/interfaces/IMerkleTree.sol";
import {ISharedLiquidityLedger} from "src/interfaces/ISharedLiquidityLedger.sol";
import {console2} from "forge-std/Console2.sol";

contract SharedLiquidityLedger is ISharedLiquidityLedger {
    IMerkleTree public stateTree;
    IZKVerifier public zkVerifier;

    uint256 public constant TREE_DEPTH = 20;

    mapping(uint256 => bytes32) public liquidityStates; // Mapping of chainId to stateRoot
    mapping(uint256 => bytes) public zkProofs; // Mapping of chainId to proof

    event LiquidityStateUpdated(uint256 indexed chainId, bytes32 stateRoot, bytes proof);

    constructor(address _zkVerifier, address _stateTree) {
        zkVerifier = IZKVerifier(_zkVerifier);
        stateTree = IMerkleTree(_stateTree);
    }

    /// @notice Gets the Merkle root of the state tree
    function getStateRoot() public view returns (bytes32) {
        return stateTree.getMerkleRoot();
    }

    /// @notice Updates the liquidity state tree with a new state root
    /// @param chainId The chain ID
    /// @param newStateRoot The new state root
    /// @param proof The 32-byte GKR proof
    /// @param previousProofs Array of previous 32-byte GKR proofs
    /// @param amount0 Public signal: amount0 from caller_delta
    /// @param amount1 Public signal: amount1 from caller_delta
    function updateLiquidityState(
        uint256 chainId,
        bytes32 newStateRoot,
        bytes calldata proof,
        bytes[] calldata previousProofs,
        int256 amount0,
        int256 amount1
    ) external override {
        // Verify GKR proof
        require(
            zkVerifier.verifyLiquidityProof(proof, previousProofs, uint256(amount0), uint256(amount1)),
            "SharedLiquidityLedger: invalid GKR proof"
        );

        _validateStateRoot(newStateRoot);
        stateTree.insert(newStateRoot);

        liquidityStates[chainId] = newStateRoot;
        zkProofs[chainId] = proof; // Store the proof (32 bytes)

        emit LiquidityStateUpdated(chainId, newStateRoot, proof);
    }

    /// @notice Fetches the latest state root for a chain
    function getLatestLiquidityState(uint256 chainId) external view returns (bytes32) {
        return liquidityStates[chainId];
    }

    /// @notice Fetches the proof for the latest state root for a chain
    function getLatestLiquidityProof(uint256 chainId) external view returns (bytes memory) {
        return zkProofs[chainId];
    }

    /// @dev Ensures that the new state root is valid
    function _validateStateRoot(bytes32 newStateRoot) internal view {
        bytes32 latestStateRoot = stateTree.getMerkleRoot();
        require(newStateRoot != latestStateRoot, "SharedLiquidityLedger: state root unchanged");
        require(newStateRoot != bytes32(0), "SharedLiquidityLedger: invalid state root");
    }

    function insert(bytes32 leaf) external override returns (bytes32 newRoot) {
        return stateTree.insert(leaf);
    }

    function getMerkleProof(uint256 index) external view override returns (bytes32[TREE_DEPTH] memory proof) {
        return stateTree.getMerkleProof(index);
    }

    function verifyProof(bytes32 leaf, bytes32[TREE_DEPTH] memory proof, bytes32 root, uint256 index)
        external
        view
        override
        returns (bool)
    {
        return stateTree.verifyProof(leaf, proof, root, index);
    }

    function getMerkleRoot() external view override returns (bytes32) {
        return stateTree.getMerkleRoot();
    }

    function getCurrentIndex() external view override returns (uint256) {
        return stateTree.getCurrentIndex();
    }

    function verifyLiquidityProof(
        bytes calldata proof,
        bytes[] calldata previousProofs,
        uint256 amount0,
        uint256 amount1
    ) external override returns (bool) {
        return zkVerifier.verifyLiquidityProof(proof, previousProofs, amount0, amount1);
    }

    function verifySwapProof(bytes calldata proof, bytes[] calldata previousProofs, uint256 amount0, uint256 amount1)
        external
        override
        returns (bool)
    {
        return zkVerifier.verifySwapProof(proof, previousProofs, amount0, amount1);
    }
}
