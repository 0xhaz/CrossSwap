// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

/// @title Interface for ZkLightClient
interface IZkLightClient {
    function sendMessage(uint16 dstChainId, address dstHook, bytes memory payload) external payable returns (uint64);

    function zkReceive(uint256 srchChainId, address srcAddress, uint64 nonce, bytes memory payload) external;

    function estimateFee(uint16 dstChainId) external view returns (uint256);
}
