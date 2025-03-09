// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

/// @title Interface for ZkLightClient
interface IZkLightClient {
    function sendMessage(uint16 dstChainId, address dstHook, bytes memory payload) external payable returns (uint64);

    function zkReceive(uint16 srcChainId, address srcAddress, uint64 nonce, bytes calldata payload) external;

    function estimateFee(uint16 dstChainId) external view returns (uint256);

    function tokenVault() external view returns (address); // Returns the token vault address

    function bridgeToken(address token, uint256 amount, uint16 dstChainId) external; // Locks and bridges tokens

    function unlockToken(address token, uint256 amount, uint16 srcChainId) external; // Unlocks/mints tokens on destination
}
