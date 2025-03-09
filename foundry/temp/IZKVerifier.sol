// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

interface IZKVerifier {
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
    ) external returns (bool);
}
