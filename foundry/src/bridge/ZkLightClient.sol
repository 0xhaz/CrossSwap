// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {console2} from "forge-std/Test.sol";
import {IZkLightClient} from "src/interfaces/IZKLightClient.sol";

/// @title Interface for zkBridge Messaging
interface IZKBridge is IZkLightClient {
    function send(uint16 dstChainId, address dstAddress, bytes memory payload)
        external
        payable
        returns (uint64 nonce);

    function estimateFee(uint16 dstChainId) external view returns (uint256 fee);
}

/// @title Interface for zkBridge Receiver
interface IZKBridgeReceiver {
    function zkReceive(uint16 srcChainId, address srcAddress, uint64 nonce, bytes calldata payload) external;
}

/// @title ZkLightClient - Facilitates cross-chain messaging with zkBridge
contract ZkLightClient is IZKBridgeReceiver {
    /// @notice Address of the zkBridge contract
    address public immutable zkBridge;

    /// @notice Event emitted when a cross-chain message is sent
    event MessageSent(
        uint16 indexed srcChainId, uint16 indexed dstChainId, address indexed destHook, uint64 nonce, bytes payload
    );

    /// @notice Event emitted when a cross-chain message is received
    event MessageReceived(uint16 indexed srcChainId, address indexed srcAddress, uint64 nonce, bytes payload);

    /// @dev Mapping to prevent duplicate messages
    mapping(bytes32 => bool) public receivedMessages;

    /// @notice Sets the zkBridge address upon deployment
    /// @param _zkBridge Address of the zkBridge contract
    constructor(address _zkBridge) {
        require(_zkBridge != address(0), "ZkLightClient: Invalid zkBridge address");
        zkBridge = _zkBridge;
    }

    /**
     * @notice Sends a cross-chain message via zkBridge
     * @dev Requires `msg.value` for relayer fee
     * @param dstChainId Destination chain ID
     * @param destHook Contract on the destination chain to receive the payload
     * @param payload Encoded payload data
     * @return nonce The message nonce
     */
    function sendMessage(uint16 dstChainId, address destHook, bytes memory payload) external payable returns (uint64) {
        require(msg.value > 0, "ZkLightClient: Relayer fee required");

        uint64 nonce = IZKBridge(zkBridge).send{value: msg.value}(dstChainId, destHook, payload);

        emit MessageSent(uint16(block.chainid), dstChainId, destHook, nonce, payload);

        return nonce;
    }

    /**
     * @notice Called by zkBridge when a cross-chain message is received
     * @dev Ensures message integrity and executes the payload
     * @param srcChainId Source chain ID
     * @param srcAddress Address that sent the message from the source chain
     * @param nonce Unique message nonce
     * @param payload Encoded payload data
     */
    function zkReceive(uint16 srcChainId, address srcAddress, uint64 nonce, bytes memory payload) external override {
        require(msg.sender == zkBridge, "ZkLightClient: Unauthorized sender");

        bytes32 messageHash = keccak256(payload);
        require(!receivedMessages[messageHash], "ZkLightClient: Message already received");

        receivedMessages[messageHash] = true;

        emit MessageReceived(srcChainId, srcAddress, nonce, payload);

        (address receiver, bytes memory messageData) = abi.decode(payload, (address, bytes));

        (bool success, bytes memory returnData) = receiver.call(
            abi.encodeWithSignature(
                "zkReceive(uint16,address,uint64,bytes)", srcChainId, srcAddress, nonce, messageData
            )
        );

        if (!success) {
            revert(string(abi.encodePacked("ZkLightClient: failed to call receiver. Error: ", returnData)));
        }
    }

    /**
     * @notice Estimates the relayer fee for a cross-chain message
     * @param dstChainId Destination chain ID
     * @return fee Estimated fee in native gas
     */
    function estimateFee(uint16 dstChainId) external view returns (uint256 fee) {
        return IZKBridge(zkBridge).estimateFee(dstChainId);
    }

    /**
     * @notice Returns the address of the token vault for bridging
     * @return vault Address of the token vault
     */
    function tokenVault() external view returns (address) {
        return IZKBridge(zkBridge).tokenVault(); // Delegate to zkBridge
    }

    /**
     * @notice Bridges tokens to the destination chain
     * @param token Address of the token to bridge
     * @param amount Amount of tokens to bridge
     * @param dstChainId Destination chain ID
     */
    function bridgeToken(address token, uint256 amount, uint16 dstChainId) external {
        IZKBridge(zkBridge).bridgeToken(token, amount, dstChainId); // Delegate to zkBridge
    }

    /**
     * @notice Unlocks or mints tokens on the destination chain
     * @param token Address of the token to unlock
     * @param amount Amount of tokens to unlock
     * @param srcChainId Source chain ID
     */
    function unlockToken(address token, uint256 amount, uint16 srcChainId) external {
        IZKBridge(zkBridge).unlockToken(token, amount, srcChainId); // Delegate to zkBridge
    }
}
