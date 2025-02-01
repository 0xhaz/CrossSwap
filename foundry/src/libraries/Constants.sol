// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {PoolId, PoolIdLibrary} from "@uniswap/v4-core/src/types/PoolId.sol";
import {Client} from "@chainlink/contracts-ccip/src/v0.8/ccip/libraries/Client.sol";

library Constants {
    bytes internal constant ZERO_BYTES = bytes("");

    /// @notice Data passed during unlocking liquidity callback, includes sender and key info
    /// @param sender Address of the sender initiating the callback
    /// @param key The pool key associated with the liquidity position
    /// @param params Parameters for modifying the liquidity position
    struct CallbackData {
        address sender;
        PoolKey key;
        IPoolManager.ModifyLiquidityParams params;
        uint256 strategyId;
        bool isCrossChainIncoming;
        bool isSwap;
        IPoolManager.SwapParams swapParams;
        bytes zkProof;
    }

    /// Struct representing a liquidity distribution strategy
    struct Strategy {
        uint256[] chainIds;
        uint256[] percentages;
        uint16[] chainSelectors;
        address[] hooks;
    }

    /// Struct to hold details of a received message
    struct Message {
        uint16 sourceChainSelector;
        address sender;
        address token0;
        uint256 amount0;
        address token1;
        uint256 amount1;
        uint24 fee;
        int24 tickSpacing;
        int24 tickLower;
        int24 tickUpper;
    }

    /// Struct to hold details of a received message for a CCIP receiver
    struct CCIPReceiveParams {
        address sender;
        address recipient;
        uint24 fee;
        int24 tickSpacing;
        int24 tickLower;
        int24 tickUpper;
        address token0Address;
        uint256 token0Amount;
        address token1Address;
        uint256 token1Amount;
        bool isSwap;
        bytes zkProof;
    }

    /// Struct to hold details of message sent to a CCIP receiver
    struct SendMessageParams {
        uint16 destinationChainSelector;
        address receiver;
        address sender;
        address token0;
        uint256 amount0;
        address token1;
        uint256 amount1;
        uint24 fee;
        int24 tickSpacing;
        int24 tickLower;
        int24 tickUpper;
        bool isSwap;
        bytes zkProof;
    }
}

library Errors {
    /// @notice Error when trying to access a message but no message exists
    error NoMessageReceived();
    /// @notice Error when the provided index is out of bounds
    error IndexOutOfBounds(uint256 providedIndex, uint256 maxIndex);
    /// @notice Error when provided message ID is not found
    error MessageIdNotExists(bytes32 messageId);
    /// @notice Error when trying to withdraw an empty amount
    error NothingToWithdraw();
    /// @notice Error when withdrawals fail
    error FailedToWithdraw(address owner, address target, uint256 value);
    /// @notice Error when contract balance is insufficient
    error InsufficientFeeTokenAmount();
}

library Events {
    /// @notice Event emitted when a strategy is added
    event StrategyAdded(
        PoolId poolId, uint256 strategyId, uint256[] chainIds, uint256[] liquidityPercentages, address[] hooks
    );

    /// @notice Event emitted when a message is sent to another chain
    /// @dev The chain selector of the destination chain
    /// @dev The address of the receiver on the destination chain
    /// @dev The message that was sent
    /// @dev The token0 amount that was sent
    /// @dev The token1 amount that was sent
    /// @dev The fee amount that was sent
    event MessageSent(
        bytes32 indexed messageId,
        uint16 indexed destinationChainSelector,
        address receiver,
        Client.EVMTokenAmount tokenAmount0,
        Client.EVMTokenAmount tokenAmount1,
        uint256 fees
    );

    /// @notice Event emitted when a message is received from another chain
    /// @dev The chain selector of the source chain
    /// @dev The address of the sender on the source chain
    /// @dev The message that was received
    /// @dev The token amount that was received
    event MessageReceived(
        bytes payload,
        uint64 srcChainId,
        address sender,
        address token0,
        uint256 amount0,
        address token1,
        uint256 amount1
    );

    /// @notice Event emitted when there is a change in existing strategy
    /// @dev The poolId of the pool
    /// @dev The strategyId of the strategy
    event StrategyUpdated(PoolId poolId, uint256 strategyId);

    /// @notice Event emitted when a strategy is removed
    /// @dev The poolId of the pool
    /// @dev The strategyId of the strategy
    event StrategyRemoved(PoolId poolId, uint256 strategyId);
}
