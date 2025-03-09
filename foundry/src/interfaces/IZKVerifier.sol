// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.24;

interface IZKVerifier {
    function verifyLiquidityProof(
        bytes calldata proof,
        bytes[] calldata previousProofs,
        uint256 amount0,
        uint256 amount1
    ) external returns (bool);

    function verifySwapProof(bytes calldata proof, bytes[] calldata previousProofs, uint256 amount0, uint256 amount1)
        external
        returns (bool);
}
