// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {BaseHook} from "v4-periphery/src/base/hooks/BaseHook.sol";
import {SafeCast} from "@uniswap/v4-core/src/libraries/SafeCast.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {CurrencyLibrary, Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {CurrencySettler} from "@uniswap/v4-core/test/utils/CurrencySettler.sol";
import {TickMath} from "@uniswap/v4-core/src/libraries/TickMath.sol";
import {BalanceDelta, BalanceDeltaLibrary} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
import {IERC20Minimal} from "@uniswap/v4-core/src/interfaces/external/IERC20Minimal.sol";
import {PoolId, PoolIdLibrary} from "@uniswap/v4-core/src/types/PoolId.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {FullMath} from "@uniswap/v4-core/src/libraries/FullMath.sol";
import {FixedPoint96} from "@uniswap/v4-core/src/libraries/FixedPoint96.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {BeforeSwapDelta, BeforeSwapDeltaLibrary} from "@uniswap/v4-core/src/types/BeforeSwapDelta.sol";
import {LiquidityAmounts} from "@uniswap/v4-core/test/utils/LiquidityAmounts.sol";
import {Constants, Errors, Events} from "src/libraries/Constants.sol";
import {IZkLightClient} from "src/interfaces/IZKLightClient.sol";
import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";
import {CrossSwapCore} from "src/core/CrossSwapCore.sol";
import {console2} from "forge-std/Test.sol";

/// @title CrossSwap - A Uniswap v4 Hook for Cross-Chain Liquidity and Swaps
/// @notice Enables privacy-preserving cross-chain liquidity management and swaps using zk proofs.
/// @dev Integrates Poseidon hashing for Merkle tree state roots and GKR proofs (placeholder in ZKVerifier).
///      Features reentrancy protection, 24-hour replay protection, and sender validation in zkReceive.
contract CrossSwap is CrossSwapCore {
    using CurrencyLibrary for Currency;
    using CurrencySettler for Currency;
    using PoolIdLibrary for PoolKey;
    using SafeCast for uint256;
    using SafeCast for uint128;
    using StateLibrary for IPoolManager;

    // Reentrancy guard state
    uint256 private locked;

    /*//////////////////////////////////////////////////////////////
                           STORAGE VARIABLES
    //////////////////////////////////////////////////////////////*/

    mapping(PoolId => mapping(uint256 => Constants.Strategy)) internal strategies;
    mapping(bytes32 => Constants.CrossChainParams) public messageDetail;
    mapping(bytes32 => uint256) private receivedMessages; // messageId => timestamp for expiration

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTOR
    //////////////////////////////////////////////////////////////*/

    constructor(
        IPoolManager poolManager,
        address authorizedUser,
        address zkClient_,
        address sharedLiquidityLedger_,
        uint256 hookChainId
    ) CrossSwapCore(poolManager, authorizedUser, zkClient_, sharedLiquidityLedger_, hookChainId) {
        locked = 1; // Initialize non-reentrant
    }

    /*//////////////////////////////////////////////////////////////
                            ADMIN FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function setAuthorizedUser(address authorizedUser) external onlyAuthorizedUser {
        authorizedUser_ = authorizedUser;
    }

    function setHookChainId(uint256 hookChainId) external onlyAuthorizedUser {
        hookChainId_ = hookChainId;
    }

    /*//////////////////////////////////////////////////////////////
                             HOOK FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function beforeAddLiquidity(
        address sender,
        PoolKey calldata key,
        IPoolManager.ModifyLiquidityParams calldata params,
        bytes calldata data
    ) external override returns (bytes4) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing proof data");

        (Constants.GKRProofData memory gkrProof, uint256 strategyId) =
            abi.decode(data, (Constants.GKRProofData, uint256));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");
        require(
            strategies[key.toId()][strategyId].chainIds.length > 0 || strategyId == 0, "CrossSwap: Invalid strategyId"
        );

        PoolId poolId = key.toId();
        (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        bytes32 existingRoot = getLatestLiquidityState(hookChainId_);
        if (existingRoot == keccak256(abi.encode(gkrProof.amount0, gkrProof.amount1))) {
            return this.beforeAddLiquidity.selector;
        }

        _executeAddLiquidity(
            Constants.LiquidityParams({
                sender: sender,
                key: key,
                params: params,
                swapParams: IPoolManager.SwapParams({zeroForOne: false, amountSpecified: 0, sqrtPriceLimitX96: 0}),
                destinationChainId: uint16(hookChainId_),
                destinationHook: address(this),
                liquidity: uint256(params.liquidityDelta),
                sqrtPriceX96: sqrtPriceX96,
                gkrProof: gkrProof,
                isSwap: false,
                isCrossChain: false
            }),
            strategyId
        );

        return this.beforeAddLiquidity.selector;
    }

    function afterAddLiquidity(
        address sender,
        PoolKey calldata key,
        IPoolManager.ModifyLiquidityParams calldata,
        BalanceDelta delta,
        BalanceDelta,
        bytes calldata data
    ) external override returns (bytes4, BalanceDelta) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing proof data");

        (Constants.GKRProofData memory gkrProof, uint256 strategyId) =
            abi.decode(data, (Constants.GKRProofData, uint256));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");
        require(
            strategies[key.toId()][strategyId].chainIds.length > 0 || strategyId == 0, "CrossSwap: Invalid strategyId"
        );

        bytes32 expectedMerkleRoot = getMerkleRoot();
        bytes32 latestStateRoot = getLatestLiquidityState(hookChainId_);
        require(latestStateRoot == expectedMerkleRoot, "CrossSwap: Invalid state root");

        uint256 proofIndex = getCurrentIndex() - 1;
        bytes32[TREE_DEPTH] memory proof = getMerkleProof(proofIndex);
        require(verifyProof(expectedMerkleRoot, proof, latestStateRoot, proofIndex), "CrossSwap: Invalid Merkle proof");

        updateLiquidityState(
            hookChainId_, expectedMerkleRoot, gkrProof.proof, gkrProof.previousProofs, delta.amount0(), delta.amount1()
        );

        emit Events.MerkleRootValidated(expectedMerkleRoot);
        return (this.afterAddLiquidity.selector, delta);
    }

    function beforeRemoveLiquidity(
        address sender,
        PoolKey calldata key,
        IPoolManager.ModifyLiquidityParams calldata params,
        bytes calldata data
    ) external override returns (bytes4) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing proof data");

        (Constants.GKRProofData memory gkrProof, uint256 strategyId) =
            abi.decode(data, (Constants.GKRProofData, uint256));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");

        Constants.CrossChainParams memory receivedMessage = Constants.CrossChainParams({
            sourceChainId: uint16(hookChainId_),
            destinationChainId: uint16(hookChainId_),
            sender: sender,
            destinationHook: address(this),
            key: key,
            amount0: uint256(params.liquidityDelta),
            amount1: 0,
            tickLower: params.tickLower,
            tickUpper: params.tickUpper,
            isSwap: false,
            zkProof: data,
            strategyId: strategyId
        });

        _processLiquidity(receivedMessage, data, strategyId);
        return this.beforeRemoveLiquidity.selector;
    }

    function afterRemoveLiquidity(
        address sender,
        PoolKey calldata,
        IPoolManager.ModifyLiquidityParams calldata,
        BalanceDelta delta,
        BalanceDelta,
        bytes calldata data
    ) external override returns (bytes4, BalanceDelta) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing proof data");

        (Constants.GKRProofData memory gkrProof, uint256 strategyId) =
            abi.decode(data, (Constants.GKRProofData, uint256));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");

        bytes32 latestStateRoot = getLatestLiquidityState(hookChainId_);
        bytes32 expectedMerkleRoot = getMerkleRoot();
        require(latestStateRoot == expectedMerkleRoot, "CrossSwap: Invalid state root");

        uint256 proofIndex = getCurrentIndex() - 1;
        bytes32[TREE_DEPTH] memory proof = getMerkleProof(proofIndex);
        require(verifyProof(expectedMerkleRoot, proof, latestStateRoot, proofIndex), "CrossSwap: Invalid Merkle proof");

        updateLiquidityState(
            hookChainId_, expectedMerkleRoot, gkrProof.proof, gkrProof.previousProofs, delta.amount0(), delta.amount1()
        );

        emit Events.MerkleRootValidated(expectedMerkleRoot);
        return (this.afterRemoveLiquidity.selector, delta);
    }

    function beforeSwap(
        address sender,
        PoolKey calldata key,
        IPoolManager.SwapParams calldata params,
        bytes calldata data
    ) external override returns (bytes4, BeforeSwapDelta, uint24) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing proof data");

        (Constants.GKRProofData memory gkrProof, uint256 strategyId) =
            abi.decode(data, (Constants.GKRProofData, uint256));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");
        require(
            strategies[key.toId()][strategyId].chainIds.length > 0 || strategyId == 0, "CrossSwap: Invalid strategyId"
        );

        Constants.LiquidityParams memory swapParams = Constants.LiquidityParams({
            sender: sender,
            key: key,
            params: IPoolManager.ModifyLiquidityParams({liquidityDelta: 0, tickLower: 0, tickUpper: 0, salt: 0}),
            swapParams: params,
            destinationChainId: uint16(hookChainId_),
            destinationHook: address(this),
            liquidity: 0,
            sqrtPriceX96: 0,
            gkrProof: gkrProof,
            isSwap: true,
            isCrossChain: false
        });

        _executePrivacySwap(swapParams, strategyId);
        return (this.beforeSwap.selector, BeforeSwapDeltaLibrary.ZERO_DELTA, 0);
    }

    function afterSwap(
        address,
        PoolKey calldata key,
        IPoolManager.SwapParams calldata,
        BalanceDelta delta,
        bytes calldata data
    ) external override returns (bytes4, int128) {
        require(msg.sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing proof data");

        (Constants.GKRProofData memory gkrProof, uint256 strategyId) =
            abi.decode(data, (Constants.GKRProofData, uint256));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");
        require(
            strategies[key.toId()][strategyId].chainIds.length > 0 || strategyId == 0, "CrossSwap: Invalid strategyId"
        );

        bytes32 newStateRoot = getMerkleRoot();
        require(newStateRoot != bytes32(0), "CrossSwap: Invalid state root");

        updateLiquidityState(
            hookChainId_, newStateRoot, gkrProof.proof, gkrProof.previousProofs, delta.amount0(), delta.amount1()
        );

        emit Events.MerkleRootValidated(newStateRoot);
        return (this.afterSwap.selector, 0);
    }

    /*//////////////////////////////////////////////////////////////
                           EXTERNAL FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function addStrategy(
        PoolId poolId,
        uint256 strategyId,
        uint256[] memory chainIds,
        uint256[] memory liquidityPercentages,
        address[] memory hooks
    ) external onlyAuthorizedUser {
        require(strategies[poolId][strategyId].chainIds.length == 0, "CrossSwap: Strategy ID already in use");
        require(
            chainIds.length == liquidityPercentages.length && chainIds.length == hooks.length,
            "CrossSwap: Array length mismatch"
        );

        uint256 totalLiquidityPercentage;
        for (uint256 i; i < liquidityPercentages.length;) {
            unchecked {
                totalLiquidityPercentage += liquidityPercentages[i];
                i++;
            }
        }
        require(totalLiquidityPercentage == 100, "CrossSwap: Liquidity percentages must sum to 100");

        strategies[poolId][strategyId] =
            Constants.Strategy({chainIds: chainIds, percentages: liquidityPercentages, hooks: hooks});

        emit Events.StrategyAdded(poolId, strategyId, chainIds, liquidityPercentages, hooks);
    }

    function updateStrategy(
        PoolId poolId,
        uint256 strategyId,
        uint256[] memory chainIds,
        uint256[] memory liquidityPercentages,
        address[] memory hooks
    ) external onlyAuthorizedUser {
        require(strategies[poolId][strategyId].chainIds.length > 0, "CrossSwap: Strategy ID does not exist");
        require(
            chainIds.length == liquidityPercentages.length && chainIds.length == hooks.length,
            "CrossSwap: Array length mismatch"
        );

        uint256 totalLiquidityPercentage;
        for (uint256 i; i < liquidityPercentages.length;) {
            unchecked {
                totalLiquidityPercentage += liquidityPercentages[i];
                i++;
            }
        }
        require(totalLiquidityPercentage == 100, "CrossSwap: Liquidity percentages must sum to 100");

        strategies[poolId][strategyId] =
            Constants.Strategy({chainIds: chainIds, percentages: liquidityPercentages, hooks: hooks});
        emit Events.StrategyUpdated(poolId, strategyId);
    }

    function removeStrategy(PoolId poolId, uint256 strategyId) external onlyAuthorizedUser {
        require(strategies[poolId][strategyId].chainIds.length > 0, "CrossSwap: Strategy ID does not exist");
        delete strategies[poolId][strategyId];
        emit Events.StrategyRemoved(poolId, strategyId);
    }

    function setZkClient(address zkClientAddress) external onlyAuthorizedUser {
        require(zkClientAddress != address(0), "CrossSwap: Invalid zkClient address");
        zkClient = IZkLightClient(zkClientAddress);
    }

    function getZkClient() external view returns (address) {
        return address(zkClient);
    }

    /*//////////////////////////////////////////////////////////////
                           CALLBACK FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _unlockCallback(bytes calldata rawData) internal override onlyPoolManager returns (bytes memory) {
        Constants.CallbackData memory data = abi.decode(rawData, (Constants.CallbackData));
        PoolKey memory key = data.key;

        bool isCrossChain = _isCrossChain(uint16(data.strategyId));
        bool isSwap = _isSwap(data.params);

        (Constants.GKRProofData memory gkrProof,) = abi.decode(data.params, (Constants.GKRProofData, uint256));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");

        if (isCrossChain) {
            _handleCrossChain(data.sender, key, data.params, isSwap, gkrProof, hookChainId_, data.strategyId);
        } else {
            _handleLocalTransaction(key, data.sender, data.params, data.strategyId, isSwap, gkrProof);
        }

        return abi.encode(BalanceDeltaLibrary.ZERO_DELTA);
    }

    /*//////////////////////////////////////////////////////////////
                             CROSS CHAIN FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _sendMessage(Constants.CrossChainParams memory params) internal returns (bytes32 messageId) {
        bytes memory payload = abi.encode(params);
        uint256 estimateFeeValue = estimateFee(params.destinationChainId);
        require(msg.value >= estimateFeeValue, "CrossSwap: Insufficient fee");

        sendMessage(params.destinationChainId, params.destinationHook, payload);
        messageId = keccak256(payload);
        emit Events.MessageSent(messageId, params.destinationChainId, params.destinationHook, estimateFeeValue);
        return messageId;
    }

    function zkReceive(uint16 srcChainId, address srcAddress, uint64 nonce, bytes calldata payload) external override {
        require(msg.sender == address(zkClient), "CrossSwap: Unauthorized sender");
        require(locked == 1, "CrossSwap: Reentrancy detected"); // Reentrancy check
        locked = 2;

        // Track processed messages to prevent replays
        bytes32 messageId = keccak256(abi.encodePacked(srcChainId, srcAddress, nonce, payload));
        require(
            receivedMessages[messageId] == 0 || block.timestamp > receivedMessages[messageId] + 24 hours,
            "CrossSwap: Message already processed"
        );
        receivedMessages[messageId] = block.timestamp;

        Constants.CrossChainParams memory receivedMessage = abi.decode(payload, (Constants.CrossChainParams));
        Constants.GKRProofData memory gkrProof = abi.decode(receivedMessage.zkProof, (Constants.GKRProofData));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");

        // Validate srcAddress matches receivedMessage.sender
        require(srcAddress == receivedMessage.sender, "CrossSwap: Sender mismatch");

        PoolKey memory key = PoolKey({
            currency0: Currency.wrap(Currency.unwrap(receivedMessage.key.currency0)),
            currency1: Currency.wrap(Currency.unwrap(receivedMessage.key.currency1)),
            fee: receivedMessage.key.fee,
            tickSpacing: receivedMessage.key.tickSpacing,
            hooks: IHooks(address(this))
        });

        bytes memory params;
        if (receivedMessage.isSwap) {
            params = abi.encode(
                IPoolManager.SwapParams({
                    zeroForOne: false,
                    amountSpecified: int256(receivedMessage.amount0),
                    sqrtPriceLimitX96: 0
                })
            );
        } else {
            params = abi.encode(
                IPoolManager.ModifyLiquidityParams({
                    liquidityDelta: int256(receivedMessage.amount0),
                    tickLower: receivedMessage.tickLower,
                    tickUpper: receivedMessage.tickUpper,
                    salt: bytes32(0)
                })
            );
        }

        if (receivedMessage.isSwap) {
            Constants.LiquidityParams memory swapParams = Constants.LiquidityParams({
                sender: receivedMessage.sender,
                key: key,
                params: IPoolManager.ModifyLiquidityParams({liquidityDelta: 0, tickLower: 0, tickUpper: 0, salt: 0}),
                swapParams: IPoolManager.SwapParams({
                    zeroForOne: false,
                    amountSpecified: int256(receivedMessage.amount0),
                    sqrtPriceLimitX96: 0
                }),
                destinationChainId: srcChainId,
                destinationHook: address(this),
                liquidity: 0,
                sqrtPriceX96: 0,
                gkrProof: gkrProof,
                isSwap: true,
                isCrossChain: false
            });
            _executePrivacySwap(swapParams, receivedMessage.strategyId);
        } else {
            _processLiquidity(receivedMessage, receivedMessage.zkProof, receivedMessage.strategyId);
            if (receivedMessage.amount0 > 0) {
                zkClient.unlockToken(
                    Currency.unwrap(receivedMessage.key.currency0), receivedMessage.amount0, srcChainId
                );
            }
            if (receivedMessage.amount1 > 0) {
                zkClient.unlockToken(
                    Currency.unwrap(receivedMessage.key.currency1), receivedMessage.amount1, srcChainId
                );
            }
        }

        _handleCrossChain(
            receivedMessage.sender,
            key,
            params,
            receivedMessage.isSwap,
            gkrProof,
            uint256(srcChainId),
            receivedMessage.strategyId
        );

        emit Events.MessageReceived(
            payload,
            srcChainId,
            srcAddress,
            receivedMessage.sender,
            Currency.unwrap(receivedMessage.key.currency0),
            receivedMessage.amount0,
            Currency.unwrap(receivedMessage.key.currency1),
            receivedMessage.amount1,
            nonce
        );

        // Log for debugging
        console2.log("Processed Message - SrcAddress:", srcAddress);
        console2.log("Nonce:", nonce);

        locked = 1; // Unlock
    }

    /*//////////////////////////////////////////////////////////////
                            HELPER FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _processLiquidity(
        Constants.CrossChainParams memory receivedMessage,
        bytes memory proofData,
        uint256 strategyId
    ) private {
        Constants.GKRProofData memory gkrProof = abi.decode(proofData, (Constants.GKRProofData));
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");

        PoolKey memory key = receivedMessage.key;
        PoolId poolId = key.toId();
        (uint160 currentSqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        uint160 lowerSqrtPriceX96 = TickMath.getSqrtPriceAtTick(receivedMessage.tickLower);
        uint160 upperSqrtPriceX96 = TickMath.getSqrtPriceAtTick(receivedMessage.tickUpper);

        uint128 liquidity = LiquidityAmounts.getLiquidityForAmounts(
            currentSqrtPriceX96, lowerSqrtPriceX96, upperSqrtPriceX96, receivedMessage.amount0, receivedMessage.amount1
        );

        IPoolManager.ModifyLiquidityParams memory modifyParams = IPoolManager.ModifyLiquidityParams({
            liquidityDelta: int256(uint256(liquidity)),
            tickLower: receivedMessage.tickLower,
            tickUpper: receivedMessage.tickUpper,
            salt: bytes32(0)
        });

        BalanceDelta delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: msg.sender,
                        key: key,
                        params: abi.encode(modifyParams),
                        strategyId: strategyId
                    })
                )
            ),
            (BalanceDelta)
        );

        receivedMessage.amount0 = receivedMessage.amount0 > uint256(uint128(delta.amount0()))
            ? receivedMessage.amount0 - uint256(uint128(delta.amount0()))
            : 0;
        receivedMessage.amount1 = receivedMessage.amount1 > uint256(uint128(delta.amount1()))
            ? receivedMessage.amount1 - uint256(uint128(delta.amount1()))
            : 0;
        _refundRemainingTokens(receivedMessage);
    }

    function _refundRemainingTokens(Constants.CrossChainParams memory params) private {
        address token0 = Currency.unwrap(params.key.currency0);
        address token1 = Currency.unwrap(params.key.currency1);

        if (params.amount0 > 0) {
            require(IERC20Minimal(token0).transfer(params.sender, params.amount0), "CrossSwap: Refund token0 failed");
        }
        if (params.amount1 > 0) {
            require(IERC20Minimal(token1).transfer(params.sender, params.amount1), "CrossSwap: Refund token1 failed");
        }
    }

    function _calculateLiquidityAmounts(Constants.Strategy storage strategy, uint256 liquidityAmount)
        internal
        view
        returns (uint256[] memory liquidityAmounts)
    {
        liquidityAmounts = new uint256[](strategy.chainIds.length);
        uint256 totalAllocated;

        for (uint256 i; i < strategy.percentages.length;) {
            uint256 amount = (liquidityAmount * strategy.percentages[i]) / 100;
            liquidityAmounts[i] = amount;
            unchecked {
                totalAllocated += amount;
                i++;
            }
        }
        uint256 remaining = liquidityAmount - totalAllocated;
        if (remaining > 0) {
            for (uint256 i; i < strategy.chainIds.length && remaining > 0;) {
                uint256 adjust = remaining / (strategy.chainIds.length - i);
                liquidityAmounts[i] += adjust;
                unchecked {
                    remaining -= adjust;
                    i++;
                }
            }
        }
    }

    function _calculateTokenAmounts(
        IPoolManager.ModifyLiquidityParams memory params,
        uint256 liquidity,
        uint160 sqrtPriceX96
    ) internal pure returns (uint256 amount0, uint256 amount1) {
        uint160 sqrtPriceAX96 = TickMath.getSqrtPriceAtTick(params.tickLower);
        uint160 sqrtPriceBX96 = TickMath.getSqrtPriceAtTick(params.tickUpper);

        if (sqrtPriceX96 <= sqrtPriceAX96) {
            amount0 = FullMath.mulDiv(liquidity << 96, sqrtPriceBX96 - sqrtPriceAX96, sqrtPriceBX96) / sqrtPriceAX96;
            amount1 = 0;
        } else if (sqrtPriceX96 < sqrtPriceBX96) {
            amount0 = FullMath.mulDiv(liquidity << 96, sqrtPriceBX96 - sqrtPriceX96, sqrtPriceBX96) / sqrtPriceX96;
            amount1 = FullMath.mulDiv(liquidity, sqrtPriceX96 - sqrtPriceAX96, FixedPoint96.Q96);
        } else {
            amount0 = 0;
            amount1 = FullMath.mulDiv(liquidity, sqrtPriceBX96 - sqrtPriceAX96, FixedPoint96.Q96);
        }
    }

    function _takeDeltas(address sender, PoolKey memory key, BalanceDelta delta) internal {
        if (delta.amount0() < 0) {
            poolManager.take(key.currency0, sender, uint256(uint128(-delta.amount0())));
        }
        if (delta.amount1() < 0) {
            poolManager.take(key.currency1, sender, uint256(uint128(-delta.amount1())));
        }
    }

    function _settleDeltas(address sender, PoolKey memory key, BalanceDelta delta) internal {
        if (delta.amount0() > 0) {
            _settleDelta(sender, key.currency0, uint128(delta.amount0()));
        }
        if (delta.amount1() > 0) {
            _settleDelta(sender, key.currency1, uint128(delta.amount1()));
        }
    }

    function _settleDelta(address sender, Currency currency, uint128 amount) internal {
        currency.settle(poolManager, sender, amount, false);
    }

    function _executePrivacySwap(Constants.LiquidityParams memory swapParams, uint256 strategyId) private {
        require(swapParams.isSwap, "CrossSwap: Not a swap");

        (uint256 amount0, uint256 amount1) = _calculateSwapAmounts(swapParams.swapParams);

        BalanceDelta swapDelta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: swapParams.sender,
                        key: swapParams.key,
                        params: abi.encode(swapParams.params),
                        strategyId: strategyId
                    })
                )
            ),
            (BalanceDelta)
        );

        _settleDeltas(swapParams.sender, swapParams.key, swapDelta);

        Constants.CrossChainParams memory crossChainParams = Constants.CrossChainParams({
            sourceChainId: uint16(hookChainId_),
            destinationChainId: swapParams.destinationChainId,
            sender: swapParams.sender,
            destinationHook: swapParams.destinationHook,
            key: swapParams.key,
            amount0: amount0,
            amount1: amount1,
            tickLower: 0,
            tickUpper: 0,
            isSwap: true,
            zkProof: abi.encode(swapParams.gkrProof),
            strategyId: strategyId
        });

        if (swapParams.isCrossChain) {
            uint256 fee = zkClient.estimateFee(swapParams.destinationChainId);
            require(msg.value >= fee, "CrossSwap: Insufficient cross-chain fee");
            _transferCrossChain(crossChainParams);
        }
    }

    function _executeAddLiquidity(Constants.LiquidityParams memory liquidityParams, uint256 strategyId)
        internal
        returns (BalanceDelta delta)
    {
        (uint256 amount0, uint256 amount1) =
            _calculateTokenAmounts(liquidityParams.params, liquidityParams.liquidity, liquidityParams.sqrtPriceX96);

        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: liquidityParams.sender,
                        key: liquidityParams.key,
                        params: abi.encode(liquidityParams.params),
                        strategyId: strategyId
                    })
                )
            ),
            (BalanceDelta)
        );

        _settleDeltas(liquidityParams.sender, liquidityParams.key, delta);

        bytes32 newStateRoot = PoseidonHasherLibrary.hashSingle(bytes32(amount0), bytes32(amount1));
        _updateMerkleTree(uint16(hookChainId_), newStateRoot);

        emit Events.MerkleRootUpdated(uint16(hookChainId_), newStateRoot);

        updateLiquidityState(
            liquidityParams.destinationChainId,
            newStateRoot,
            liquidityParams.gkrProof.proof,
            liquidityParams.gkrProof.previousProofs,
            int256(amount0),
            int256(amount1)
        );

        if (liquidityParams.isCrossChain) {
            uint256 fee = zkClient.estimateFee(liquidityParams.destinationChainId);
            require(msg.value >= fee, "CrossSwap: Insufficient cross-chain fee");
            Constants.CrossChainParams memory crossChainParams = Constants.CrossChainParams({
                sourceChainId: uint16(hookChainId_),
                destinationChainId: liquidityParams.destinationChainId,
                sender: liquidityParams.sender,
                destinationHook: liquidityParams.destinationHook,
                key: liquidityParams.key,
                amount0: amount0,
                amount1: amount1,
                tickLower: liquidityParams.params.tickLower,
                tickUpper: liquidityParams.params.tickUpper,
                isSwap: false,
                zkProof: abi.encode(liquidityParams.gkrProof),
                strategyId: strategyId
            });
            _transferCrossChain(crossChainParams);
        }

        return delta;
    }

    function _updateMerkleTree(uint16 chainId, bytes32 newStateRoot) private {
        bytes32 latestStateRoot = getLatestLiquidityState(chainId);
        require(newStateRoot != latestStateRoot, "CrossSwap: State root unchanged");

        insert(newStateRoot);
        uint256 newLeafIndex = getCurrentIndex() - 1;
        bytes32[TREE_DEPTH] memory proof = getMerkleProof(newLeafIndex);
        bytes32 merkleRoot = getMerkleRoot();
        require(verifyProof(newStateRoot, proof, merkleRoot, newLeafIndex), "CrossSwap: Invalid Merkle proof");
    }

    function _handleCrossChain(
        address sender,
        PoolKey memory key,
        bytes memory params,
        bool isSwap,
        Constants.GKRProofData memory gkrProof,
        uint256 hookChainId,
        uint256 strategyId
    ) private {
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");

        if (isSwap) {
            IPoolManager.SwapParams memory swapParams = abi.decode(params, (IPoolManager.SwapParams));
            Constants.LiquidityParams memory liquidityParams = Constants.LiquidityParams({
                sender: sender,
                key: key,
                params: IPoolManager.ModifyLiquidityParams({liquidityDelta: 0, tickLower: 0, tickUpper: 0, salt: bytes32(0)}),
                swapParams: swapParams,
                destinationChainId: uint16(hookChainId),
                destinationHook: address(0),
                liquidity: 0,
                sqrtPriceX96: 0,
                gkrProof: gkrProof,
                isSwap: true,
                isCrossChain: true
            });
            _executePrivacySwap(liquidityParams, strategyId);
        } else {
            IPoolManager.ModifyLiquidityParams memory modifyParams =
                abi.decode(params, (IPoolManager.ModifyLiquidityParams));
            Constants.CrossChainParams memory receivedMessage = Constants.CrossChainParams({
                sourceChainId: uint16(hookChainId),
                destinationChainId: 0,
                sender: sender,
                destinationHook: address(0),
                key: key,
                amount0: uint256(modifyParams.liquidityDelta),
                amount1: 0,
                tickLower: modifyParams.tickLower,
                tickUpper: modifyParams.tickUpper,
                isSwap: false,
                zkProof: abi.encode(gkrProof),
                strategyId: strategyId
            });
            _processLiquidity(receivedMessage, abi.encode(gkrProof), strategyId);
        }
    }

    function _handleLocalTransaction(
        PoolKey memory key,
        address sender,
        bytes memory params,
        uint256 strategyId,
        bool isSwap,
        Constants.GKRProofData memory gkrProof
    ) private {
        require(gkrProof.proof.length == 32, "CrossSwap: Invalid proof length");

        PoolId poolId = key.toId();
        require(strategies[poolId][strategyId].chainIds.length > 0 || strategyId == 0, "CrossSwap: Invalid strategyId");

        IPoolManager.ModifyLiquidityParams memory modifyParams =
            abi.decode(params, (IPoolManager.ModifyLiquidityParams));
        Constants.Strategy storage strategy = strategies[poolId][strategyId];

        uint256[] memory liquidityAmounts = _calculateLiquidityAmounts(strategy, uint256(modifyParams.liquidityDelta));
        (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        for (uint256 i; i < strategy.chainIds.length;) {
            uint256 liquidity = liquidityAmounts[i];
            uint16 destinationChainId = uint16(strategy.chainIds[i]);
            address destinationHook = strategy.hooks[i];
            bool isLocal = destinationChainId == uint16(hookChainId_);

            BalanceDelta delta;
            if (isSwap) {
                IPoolManager.SwapParams memory swapParams = abi.decode(params, (IPoolManager.SwapParams));
                Constants.LiquidityParams memory liquidityParams = Constants.LiquidityParams({
                    sender: sender,
                    key: key,
                    params: modifyParams,
                    swapParams: swapParams,
                    destinationChainId: destinationChainId,
                    destinationHook: destinationHook,
                    liquidity: 0,
                    sqrtPriceX96: sqrtPriceX96,
                    gkrProof: gkrProof,
                    isSwap: true,
                    isCrossChain: !isLocal
                });
                _executePrivacySwap(liquidityParams, strategyId);
            } else {
                Constants.LiquidityParams memory liquidityParams = Constants.LiquidityParams({
                    sender: sender,
                    key: key,
                    params: modifyParams,
                    swapParams: IPoolManager.SwapParams({zeroForOne: false, amountSpecified: 0, sqrtPriceLimitX96: 0}),
                    destinationChainId: destinationChainId,
                    destinationHook: destinationHook,
                    liquidity: liquidity,
                    sqrtPriceX96: sqrtPriceX96,
                    gkrProof: gkrProof,
                    isSwap: false,
                    isCrossChain: !isLocal
                });
                delta = _executeAddLiquidity(liquidityParams, strategyId);
            }
            if (!isLocal) {
                _takeDeltas(sender, key, delta);
            }
            unchecked {
                i++;
            }
        }
    }

    function _isCrossChain(uint16 destinationChainId) private pure returns (bool) {
        return destinationChainId != 0;
    }

    function _isSwap(bytes memory params) private pure returns (bool) {
        return params.length == 32; // Assumes SwapParams encoding length
    }

    function _calculateSwapAmounts(IPoolManager.SwapParams memory params)
        private
        pure
        returns (uint256 amount0, uint256 amount1)
    {
        if (params.zeroForOne) {
            amount0 = params.amountSpecified > 0 ? uint256(params.amountSpecified) : uint256(-params.amountSpecified);
            amount1 = 0;
        } else {
            amount0 = 0;
            amount1 = params.amountSpecified > 0 ? uint256(params.amountSpecified) : uint256(-params.amountSpecified);
        }
    }

    function _transferCrossChain(Constants.CrossChainParams memory transferParams) private {
        require(locked == 1, "CrossSwap: Reentrancy detected"); // Reentrancy check
        locked = 2;

        address zkBridgeVault = zkClient.tokenVault();
        uint256 fee = zkClient.estimateFee(transferParams.destinationChainId);
        require(msg.value >= fee, "CrossSwap: Insufficient cross-chain fee");

        if (transferParams.amount0 > 0) {
            IERC20Minimal token0 = IERC20Minimal(Currency.unwrap(transferParams.key.currency0));
            require(
                token0.transferFrom(transferParams.sender, zkBridgeVault, transferParams.amount0),
                "CrossSwap: Token0 transfer failed"
            );
            require(token0.approve(zkBridgeVault, transferParams.amount0), "CrossSwap: Token0 approval failed");
            zkClient.bridgeToken(
                Currency.unwrap(transferParams.key.currency0), transferParams.amount0, transferParams.destinationChainId
            );
        }
        if (transferParams.amount1 > 0) {
            IERC20Minimal token1 = IERC20Minimal(Currency.unwrap(transferParams.key.currency1));
            require(
                token1.transferFrom(transferParams.sender, zkBridgeVault, transferParams.amount1),
                "CrossSwap: Token1 transfer failed"
            );
            require(token1.approve(zkBridgeVault, transferParams.amount1), "CrossSwap: Token1 approval failed");
            zkClient.bridgeToken(
                Currency.unwrap(transferParams.key.currency1), transferParams.amount1, transferParams.destinationChainId
            );
        }

        _sendMessage(transferParams);

        locked = 1; // Unlock
    }
}
