// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {console2} from "forge-std/Test.sol";

contract ZkLightClient {
    /// @notice Emitted when a cross-chain message is sent
    event MessageSent(uint16 indexed dstChainId, bytes indexed receiver, uint64 nonce, bytes payload);

    /// @notice Emitted when a cross-chain message is received
    event MessageReceived(uint16 indexed srcChainId, bytes indexed sender, uint64 nonce, bytes payload);

    /// @dev Simulated nonce for tracking messages
    uint64 public nonceCounter;

    /// @dev Mapping to store received messages for debugging
    mapping(bytes32 => bool) public receivedMessages;

    constructor() {
        nonceCounter = 1;
    }

    /**
     * @notice Send a message to another chain
     * @param dstChainId The destination chain ID
     * @param receiver The encoded address of the receiver on the destination chain
     * @param timestamp a unique timestamp used as nonce
     * @param payload The cross-chain message payload
     */
    function send(uint16 dstChainId, bytes memory receiver, uint64 timestamp, bytes memory payload) external {
        nonceCounter++;

        emit MessageSent(dstChainId, receiver, timestamp, payload);
    }

    /**
     * @notice Receive a message from another chain
     * @param srcChainId The source chain ID
     * @param sender The encoded address of the sender on the source chain
     * @param timestamp a unique timestamp used as nonce
     * @param payload The cross-chain message payload
     */
    function receiveMessage(uint16 srcChainId, bytes memory sender, uint64 timestamp, bytes memory payload) external {
        bytes32 messageHash = keccak256(payload);
        require(!receivedMessages[messageHash], "ZkLightClient: message already received");
        receivedMessages[messageHash] = true;

        emit MessageReceived(srcChainId, sender, timestamp, payload);

        (address receiver, bytes memory messageData) = abi.decode(payload, (address, bytes));

        (bool success,) = receiver.call(abi.encodeWithSignature("zkReceive(uint16,bytes)", srcChainId, messageData));
        require(success, "ZkLightClient: failed to call receiver");
    }

    /**
     * @notice Mock function for verifying zkProof
     * @param zkProof The zkProof to verify
     * @return bool Whether the proof is valid
     */
    function verifyProof(bytes memory zkProof) external pure returns (bool) {
        return keccak256(zkProof) != keccak256(bytes("INVALID"));
    }
}
