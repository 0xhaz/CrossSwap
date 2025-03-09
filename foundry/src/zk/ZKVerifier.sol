// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.24;

import {IZKVerifier} from "src/interfaces/IZKVerifier.sol";
import {PoseidonT3} from "poseidon-solidity/PoseidonT3.sol";
import {console2} from "forge-std/Console2.sol";

contract ZKVerifier is IZKVerifier {
    uint256 constant F = 21888242871839275222246405745257275088548364400416034343698204186575808495617;

    event ProofVerified(bytes32 proofHash, bool isValid);

    function verifyLiquidityProof(
        bytes calldata proof,
        bytes[] calldata previousProofs,
        uint256 amount0,
        uint256 amount1
    ) external returns (bool) {
        require(proof.length == 32, "Invalid proof length");
        require(previousProofs.length > 0, "No previous proofs provided");
        bool isValid = _verifyProof(proof, previousProofs, amount0, amount1);
        uint256[2] memory proofInput = [uint256(bytes32(proof)), 0];
        bytes32 proofHash = bytes32(PoseidonT3.hash(proofInput));
        emit ProofVerified(proofHash, isValid);
        return isValid;
    }

    function verifySwapProof(bytes calldata proof, bytes[] calldata previousProofs, uint256 amount0, uint256 amount1)
        external
        returns (bool)
    {
        require(proof.length == 32, "Invalid proof length");
        require(previousProofs.length > 0, "No previous proofs provided");
        bool isValid = _verifyProof(proof, previousProofs, amount0, amount1);
        uint256[2] memory proofInput = [uint256(bytes32(proof)), 0];
        bytes32 proofHash = bytes32(PoseidonT3.hash(proofInput));
        emit ProofVerified(proofHash, isValid);
        return isValid;
    }

    function _verifyProof(bytes calldata proof, bytes[] calldata previousProofs, uint256 amount0, uint256 amount1)
        internal
        pure
        returns (bool)
    {
        require(proof.length == 32, "Invalid proof length");
        bytes32 proofBytes = bytes32(proof);
        uint256 proofValue = uint256(proofBytes); // Big-endian

        bytes32 prevProofBytes =
            previousProofs.length == 1 ? bytes32(previousProofs[0]) : bytes32(previousProofs[previousProofs.length - 2]);
        uint256 prevProof = uint256(prevProofBytes);

        uint256[2] memory intermediateInput;
        intermediateInput[0] = prevProof % F;
        intermediateInput[1] = amount0 % F;
        uint256 intermediateHash = PoseidonT3.hash(intermediateInput);

        uint256[2] memory finalInput;
        finalInput[0] = intermediateHash % F;
        finalInput[1] = amount1 % F;
        uint256 computedProofHash = PoseidonT3.hash(finalInput);

        console2.log("Intermediate Hash:", intermediateHash);
        console2.log("Computed Proof Hash:", computedProofHash);
        console2.log("Provided Proof (big-endian):", proofValue);

        return computedProofHash == proofValue;
    }
}
