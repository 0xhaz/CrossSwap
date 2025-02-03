// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {console2} from "forge-std/Test.sol";
import {CrossSwap} from "src/CrossSwap.sol";

contract MockZkLightClient {
    mapping(bytes32 => bool) public receivedMessages;

    event MessageReceived(uint16 srcChainId, bytes sender, uint64 timestamp, bytes payload);
    event CrossChainLiquidityReceived(uint16 srcChainId, bytes message);

    /// @notice Receives a cross-chain message and calls `zkReceive()` on the destination contract.
    function receiveMessage(uint16 srcChainId, bytes memory sender, uint64 timestamp, bytes memory payload) external {
        bytes32 messageHash = keccak256(payload);
        require(!receivedMessages[messageHash], "ZkLightClient: message already received");
        receivedMessages[messageHash] = true;

        emit MessageReceived(srcChainId, sender, timestamp, payload);

        (address receiver, bytes memory messageData) = abi.decode(payload, (address, bytes));

        console2.log(unicode"🚀 Calling zkReceive on receiver:", receiver);
        console2.log("Decoded messageData:");
        console2.logBytes(messageData);

        uint256 codeSize;
        assembly {
            codeSize := extcodesize(receiver)
        }
        require(codeSize > 0, "ZkLightClient: Receiver contract does not exist!");

        (bool success, bytes memory returnData) =
            receiver.call(abi.encodeWithSignature("zkReceive(uint16,bytes)", srcChainId, messageData));

        console2.log(unicode"✅ Call success:", success);
        if (!success) {
            console2.log(unicode"🔴 Call failed. Return data:");
            console2.logBytes(returnData);
        }
        require(success, string(abi.encodePacked("ZkLightClient: failed to call receiver. Error: ", returnData)));
    }

    /// @notice Simulates receiving a cross-chain message.
    function mockReceiveMessage(address receiver, uint16 srcChainId, bytes memory payload) external {
        uint64 fakeTimestamp = uint64(block.timestamp);
        bytes32 messageHash = keccak256(payload);

        console2.log("Expected zkClient from MockZkLightClient:", receiver);
        console2.log("Actual zkClient from CrossSwap:", CrossSwap(receiver).getZkClient());

        require(!receivedMessages[messageHash], "ZkLightClient: message already received");
        receivedMessages[messageHash] = true;

        emit MessageReceived(srcChainId, abi.encode(receiver), fakeTimestamp, payload);

        console2.log(unicode"🚀 Calling zkReceive on receiver:", receiver);
        console2.log("srcChainId:", srcChainId);
        console2.log("Decoded messageData:");
        console2.logBytes(payload);

        uint256 codeSize;
        assembly {
            codeSize := extcodesize(receiver)
        }
        require(codeSize > 0, "ZkLightClient: Receiver contract does not exist!");

        (bool success, bytes memory returnData) =
            receiver.call(abi.encodeWithSignature("zkReceive(uint16,bytes)", srcChainId, payload));

        console2.log(unicode"✅ Call success:", success);
        if (!success) {
            console2.log(unicode"🔴 Call failed. Return data:");
            console2.logBytes(returnData);
        }
        require(success, "ZkLightClient: failed to call receiver.");
    }

    function zkReceive(uint16 srcChainId, bytes memory message) external {
        emit CrossChainLiquidityReceived(srcChainId, message);
        console2.log(unicode"✅ MockReceiver: Cross-chain liquidity received from Chain", srcChainId);
    }
}

/// @notice Simple receiver contract for testing `zkReceive`
contract Receiver {
    event CrossChainLiquidityReceived(uint16 srcChainId, bytes message);

    function zkReceive(uint16 srcChainId, bytes memory message) external {
        emit CrossChainLiquidityReceived(srcChainId, message);
        console2.log(unicode"✅ Receiver: Cross-chain liquidity received from Chain", srcChainId);
    }
}
