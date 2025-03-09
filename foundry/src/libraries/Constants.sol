// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {PoolId, PoolIdLibrary} from "@uniswap/v4-core/src/types/PoolId.sol";
import {Client} from "@chainlink/contracts-ccip/src/v0.8/ccip/libraries/Client.sol";

library Constants {
    bytes internal constant ZERO_BYTES = bytes("");
    uint256 internal constant TREE_DEPTH = 20;

    /// @notice Data passed during unlocking liquidity callback, includes sender and key info
    struct CallbackData {
        address sender;
        PoolKey key;
        bytes params;
        uint256 strategyId;
    }

    /// Struct representing a liquidity distribution strategy
    struct Strategy {
        uint256[] chainIds;
        uint256[] percentages;
        address[] hooks;
    }

    /// Struct holding zk-SNARK proof data
    struct GKRProofData {
        bytes proof; // 32-byte GKR proof
        bytes[] previousProofs; // Array of previous 32-byte proofs
        int256 amount0; // Public signal: amount0
        int256 amount1; // Public signal: amount1
    }

    /// Unified struct for liquidity execution (both swaps & adding liquidity)
    struct LiquidityParams {
        address sender;
        PoolKey key;
        IPoolManager.ModifyLiquidityParams params;
        IPoolManager.SwapParams swapParams;
        uint16 destinationChainId;
        address destinationHook;
        uint256 liquidity;
        uint160 sqrtPriceX96;
        GKRProofData gkrProof;
        bool isSwap;
        bool isCrossChain;
    }

    /// Struct for cross-chain transfers and messages (combined from two previous structs)
    struct CrossChainParams {
        uint16 sourceChainId;
        uint16 destinationChainId;
        address sender;
        address destinationHook;
        PoolKey key;
        uint256 amount0;
        uint256 amount1;
        int24 tickLower;
        int24 tickUpper;
        bool isSwap;
        bytes zkProof;
        uint256 strategyId;
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
    /// @notice Error when calling the pool manager fails
    error NotPoolManager();
    /// @notice Error when calling hooks
    error HookNotImplemented();
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
    event MessageSent(bytes32 indexed messageId, uint16 indexed destinationChainId, address receiver, uint256 fees);

    /// @notice Event emitted when a message is received from another chain
    /// @dev The chain selector of the source chain
    /// @dev The address of the sender on the source chain
    /// @dev The message that was received
    /// @dev The token amount that was received
    event MessageReceived(
        bytes payload,
        uint16 indexed srcChainId,
        address indexed srcAddress,
        address sender,
        address token0,
        uint256 amount0,
        address token1,
        uint256 amount1,
        uint64 nonce
    );

    /// @notice Event emitted when there is a change in existing strategy
    /// @dev The poolId of the pool
    /// @dev The strategyId of the strategy
    event StrategyUpdated(PoolId poolId, uint256 strategyId);

    /// @notice Event emitted when a strategy is removed
    /// @dev The poolId of the pool
    /// @dev The strategyId of the strategy
    event StrategyRemoved(PoolId poolId, uint256 strategyId);

    /// @notice Event emitted when a new state root is updated
    event MerkleRootUpdated(uint16 hookChainId, bytes32 newStateRoot);

    event MerkleRootValidated(bytes32 merkleRoot);
}
