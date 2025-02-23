// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {PoolManager} from "@uniswap/v4-core/src/PoolManager.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {PoolId, PoolIdLibrary} from "@uniswap/v4-core/src/types/PoolId.sol";
import {BalanceDelta, BalanceDeltaLibrary} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
import {TickMath} from "@uniswap/v4-core/src/libraries/TickMath.sol";
import {IERC20Minimal} from "@uniswap/v4-core/src/interfaces/external/IERC20Minimal.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {FullMath} from "@uniswap/v4-core/src/libraries/FullMath.sol";
import {FixedPoint96} from "@uniswap/v4-core/src/libraries/FixedPoint96.sol";
import {CurrencyLibrary, Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {ISharedLiquidityLedger} from "src/interfaces/ISharedLiquidityLedger.sol";
import {IZKVerifier} from "src/interfaces/IZKVerifier.sol";
import {Constants, Events} from "src/libraries/Constants.sol";
import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";
import {IZkLightClient} from "src/interfaces/IZKLightClient.sol";

uint256 constant TREE_DEPTH = 32;

library CCLib {
    using CurrencyLibrary for Currency;
    using PoolIdLibrary for PoolKey;
    using StateLibrary for IPoolManager;

    /*//////////////////////////////////////////////////////////////
                        ZK VERIFICATION
    //////////////////////////////////////////////////////////////*/

    function verifyProof(IZKVerifier zkVerifier, Constants.ZkProofData memory zkProof, bool isSwap) internal view {
        require(zkProof.publicSignals.length == (isSwap ? 5 : 4), "CrossSwap: Invalid proof length");

        uint256[] memory fixedSignals = zkProof.publicSignals; // Use dynamic array directly

        if (isSwap) {
            require(
                zkVerifier.verifySwapProof(zkProof.proofA, zkProof.proofB, zkProof.proofC, fixedSignals),
                "CrossSwap: Invalid swap proof"
            );
        } else {
            require(
                zkVerifier.verifyLiquidityProof(zkProof.proofA, zkProof.proofB, zkProof.proofC, fixedSignals),
                "CrossSwap: Invalid liquidity proof"
            );
        }
    }

    /*//////////////////////////////////////////////////////////////
                        LIQUIDITY FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function executeAddLiquidity(
        uint256 hookChainId_,
        IPoolManager poolManager,
        ISharedLiquidityLedger sharedLiquidityLedger,
        address sender,
        Constants.LiquidityParams memory liquidityParams,
        IZkLightClient zkClient
    ) internal returns (BalanceDelta delta) {
        verifyProof(sharedLiquidityLedger, liquidityParams.zkProof, false);

        // Compute token amounts
        (uint256 amount0, uint256 amount1) =
            _calculateTokenAmounts(liquidityParams.params, liquidityParams.liquidity, liquidityParams.sqrtPriceX96);

        // Update Merkle tree & ensure validity
        bytes32 newStateRoot = PoseidonHasherLibrary.hashSingle(bytes32(amount0), bytes32(amount1));
        _updateMerkleTree(sharedLiquidityLedger, liquidityParams.destinationChainId, newStateRoot);

        // Store proof data for state consistency
        bytes memory zkProofEncoded = abi.encode(liquidityParams.zkProof);
        sharedLiquidityLedger.updateLiquidityState(liquidityParams.destinationChainId, newStateRoot, zkProofEncoded);

        // Transfer liquidity across chains
        if (liquidityParams.isCrossChain) {
            Constants.CrossChainParams memory crossChainData = Constants.CrossChainParams({
                sourceChainId: uint16(hookChainId_),
                destinationChainId: liquidityParams.destinationChainId,
                sender: sender,
                destinationHook: liquidityParams.destinationHook,
                key: liquidityParams.key,
                amount0: amount0,
                amount1: amount1,
                tickLower: liquidityParams.params.tickLower,
                tickUpper: liquidityParams.params.tickUpper,
                isSwap: false,
                zkProof: zkProofEncoded
            });

            _transferCrossChain(zkClient, crossChainData);
        }

        // Unlock liquidity and return delta
        delta = _unlockLiquidity(poolManager, sender, liquidityParams.key, liquidityParams.params);
    }

    function processLiquidity(
        IZKVerifier zkVerifier,
        Constants.CrossChainParams memory receivedMessage,
        Constants.ZkProofData memory zkProof,
        ISharedLiquidityLedger sharedLiquidityLedger
    ) internal {
        // Verify zk-SNARK proof for liquidity
        verifyProof(zkVerifier, zkProof, false);

        // Compute new liquidity state root
        bytes32 newStateRoot =
            PoseidonHasherLibrary.hashSingle(bytes32(receivedMessage.amount0), bytes32(receivedMessage.amount1));

        // Fetch latest recorded liqudity state
        uint16 chainId = receivedMessage.destinationChainId;
        bytes32 latestStateRoot = sharedLiquidityLedger.getLatestLiquidityState(chainId);

        // Ensure the new state root is different from the current state root
        require(newStateRoot != latestStateRoot, "CrossSwap: State root unchanged");

        // Insert new root into Merkle Tree
        sharedLiquidityLedger.insert(newStateRoot);

        // Fetch the index of the new state root
        uint256 newLeafIndex = sharedLiquidityLedger.currentIndex() - 1;

        // Ensure the latest state root is valid
        bytes32[TREE_DEPTH] memory proof = sharedLiquidityLedger.getMerkleProof(newLeafIndex);

        // Get the latest Merkle root
        bytes32 merkleRoot = sharedLiquidityLedger.getMerkleRoot();

        require(
            sharedLiquidityLedger.verifyProof(newStateRoot, proof, merkleRoot, newLeafIndex),
            "CrossSwap: Invalid state root"
        );

        // Update liquidity state
        sharedLiquidityLedger.updateLiquidityState(chainId, newStateRoot, abi.encode(zkProof));
    }

    function _transferCrossChain(IZkLightClient zkClient, Constants.CrossChainParams memory transferParams) private {
        // Transfer tokens from sender to contract
        IERC20Minimal(Currency.unwrap(transferParams.key.currency0)).transferFrom(
            transferParams.sender, address(this), transferParams.amount0
        );
        IERC20Minimal(Currency.unwrap(transferParams.key.currency1)).transferFrom(
            transferParams.sender, address(this), transferParams.amount1
        );

        // Send the cross-chain message
        sendMessage(zkClient, transferParams);
    }

    function sendMessage(IZkLightClient zkClient, Constants.CrossChainParams memory message)
        internal
        returns (bytes32 messageId)
    {
        bytes memory payload = abi.encode(message);
        // Get the required fee for sending a cross-chain message
        uint256 estimatedFee = zkClient.estimateFee(message.destinationChainId);
        require(address(this).balance >= estimatedFee, "CrossSwap: Insufficient balance");

        // Send the cross-chain message using zkBridge
        uint64 nonce =
            zkClient.sendMessage{value: estimatedFee}(message.destinationChainId, message.destinationHook, payload);

        // Generate a unique message ID using the message hash and nonce
        messageId = keccak256(abi.encodePacked(nonce, message.sourceChainId, message.destinationChainId, payload));

        emit Events.MessageSent(
            message.sourceChainId, message.destinationChainId, message.destinationHook, nonce, payload
        );
    }

    /*//////////////////////////////////////////////////////////////
                        SWAP FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _executeSwap(
        uint256 hookChainId_,
        IPoolManager poolManager,
        ISharedLiquidityLedger sharedLiquidityLedger,
        IZkLightClient zkClient,
        Constants.LiquidityParams memory swapParams
    ) internal returns (BalanceDelta delta) {
        require(swapParams.isSwap, "CrossSwap: Not a swap");

        Constants.ZkProofData memory proofData = swapParams.zkProof;

        verifyProof(sharedLiquidityLedger, proofData, true);

        (uint256 amount0, uint256 amount1) = _calculateTokenAmounts(swapParams.swapParams);

        Constants.CrossChainParams memory crossChainParams = Constants.CrossChainParams({
            sourceChainId: uint16(hookChainId_),
            destinationChainId: swapParams.destinationChainId,
            sender: msg.sender,
            destinationHook: swapParams.destinationHook,
            key: swapParams.key,
            amount0: amount0,
            amount1: amount1,
            tickLower: 0,
            tickUpper: 0,
            isSwap: true,
            zkProof: abi.encode(proofData)
        });

        if (swapParams.isCrossChain) {
            _transferCrossChain(zkClient, crossChainParams);
        }

        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: msg.sender,
                        key: swapParams.key,
                        params: abi.encode(swapParams.swapParams),
                        strategyId: 1
                    })
                )
            ),
            (BalanceDelta)
        );
    }

    function _executeSwapWithPrivacy(
        uint256 hookChainId_,
        IPoolManager poolManager,
        ISharedLiquidityLedger sharedLiquidityLedger,
        IZKVerifier zkVerifier,
        IZkLightClient zkClient,
        Constants.LiquidityParams memory swapParams
    ) private {
        require(swapParams.isSwap, "CrossSwap: Not a swap");

        Constants.ZkProofData memory proofData = swapParams.zkProof;

        verifyProof(zkVerifier, proofData, true);

        // Compute swap token amounts
        (uint256 amount0, uint256 amount1) = _calculateSwapAmounts(swapParams.swapParams);

        BalanceDelta swapDelta = _unlockLiquidity(poolManager, msg.sender, swapParams.key, swapParams.params);
        _settleDeltas(msg.sender, swapParams.key, swapDelta);

        Constants.CrossChainParams memory crossChainParams = Constants.CrossChainParams({
            sourceChainId: uint16(hookChainId_),
            destinationChainId: swapParams.destinationChainId,
            sender: msg.sender,
            destinationHook: swapParams.destinationHook,
            key: swapParams.key,
            amount0: amount0,
            amount1: amount1,
            tickLower: 0,
            tickUpper: 0,
            isSwap: true,
            zkProof: abi.encode(proofData)
        });

        if (swapParams.isCrossChain) {
            _transferCrossChain(zkClient, crossChainParams);
        }
    }

    /*//////////////////////////////////////////////////////////////
                        MERKLE TREE MANAGEMENT
    //////////////////////////////////////////////////////////////*/

    function _updateMerkleTree(ISharedLiquidityLedger sharedLiquidityLedger, uint16 chainId, bytes32 newStateRoot)
        private
    {
        bytes32 latestStateRoot = sharedLiquidityLedger.getLatestLiquidityState(chainId);
        require(newStateRoot != latestStateRoot, "CrossSwap: State root unchanged");

        sharedLiquidityLedger.stateTree().insert(newStateRoot);

        uint256 newLeafIndex = sharedLiquidityLedger.currentIndex() - 1;

        bytes32[TREE_DEPTH] memory proof = sharedLiquidityLedger.stateTree().getMerkleProof(newLeafIndex);

        bytes32 merkleRoot = sharedLiquidityLedger.stateTree().getMerkleRoot();
        require(
            sharedLiquidityLedger.stateTree().verifyProof(newStateRoot, proof, merkleRoot, newLeafIndex),
            "CrossSwap: Invalid state root"
        );
    }

    function _unlockLiquidity(
        IPoolManager poolManager,
        address sender,
        PoolKey memory key,
        IPoolManager.ModifyLiquidityParams memory params
    ) private returns (BalanceDelta delta) {
        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({sender: sender, key: key, params: abi.encode(params), strategyId: 1})
                )
            ),
            (BalanceDelta)
        );
    }

    /*//////////////////////////////////////////////////////////////
                        PRIVATE FUNCTIONS
    //////////////////////////////////////////////////////////////*/
    function _calculateTokenAmounts(
        IPoolManager.ModifyLiquidityParams memory params,
        uint256 liquidity,
        uint160 sqrtPriceX96
    ) private pure returns (uint256 amount0, uint256 amount1) {
        uint160 sqrtPriceAX96 = TickMath.getSqrtPriceAtTick(params.tickLower);
        uint160 sqrtPriceBX96 = TickMath.getSqrtPriceAtTick(params.tickUpper);

        if (sqrtPriceX96 <= sqrtPriceAX96) {
            // Current price is below the range, only token0 is needed
            amount0 = FullMath.mulDiv(liquidity << 96, sqrtPriceBX96 - sqrtPriceAX96, sqrtPriceBX96) / sqrtPriceAX96;
            amount1 = 0;
        } else if (sqrtPriceX96 < sqrtPriceBX96) {
            // Current price is within the range, both tokens are needed
            amount0 = FullMath.mulDiv(liquidity << 96, sqrtPriceBX96 - sqrtPriceX96, sqrtPriceBX96) / sqrtPriceX96;
            amount1 = FullMath.mulDiv(liquidity, sqrtPriceX96 - sqrtPriceAX96, FixedPoint96.Q96);
        } else {
            // Current price is above the range, only token1 is needed
            amount0 = 0;
            amount1 = FullMath.mulDiv(liquidity, sqrtPriceBX96 - sqrtPriceAX96, FixedPoint96.Q96);
        }
    }

    function _createPoolKey(Constants.CrossChainParams memory message) private view returns (PoolKey memory) {
        return PoolKey({
            currency0: Currency.wrap(message.token0),
            currency1: Currency.wrap(message.token1),
            fee: message.fee,
            tickSpacing: message.tickSpacing,
            hooks: IHooks(address(this))
        });
    }

    function _handleCrossChain(
        address sender,
        PoolKey memory key,
        bytes memory params,
        bool isSwap,
        Constants.ZkProofData memory zkProof,
        uint256 hookChainId_
    ) private {
        require(
            (isSwap && zkProof.publicSignals.length == 5) || (!isSwap && zkProof.publicSignals.length == 3),
            "CrossSwap: Invalid public signals length"
        );

        Constants.CrossChainParams memory receivedMessage = Constants.CrossChainParams({
            sourceChainId: uint16(hookChainId_),
            destinationChainId: 0,
            sender: sender,
            destinationHook: address(0),
            token0: Currency.unwrap(key.currency0),
            amount0: uint256(abi.decode(params, (int256))),
            token1: Currency.unwrap(key.currency1),
            amount1: 0,
            fee: key.fee,
            tickSpacing: key.tickSpacing,
            tickLower: 0,
            tickUpper: 0,
            isSwap: isSwap,
            zkProof: abi.encode(zkProof)
        });

        if (isSwap) {
            _executeSwapWithPrivacy(key, abi.decode(params, (IPoolManager.SwapParams)), zkProof);
        } else {
            _processLiquidity(receivedMessage, zkProof);
        }
    }

    function _handleLocalTransaction(
        PoolKey memory key,
        address sender,
        bytes memory params,
        uint256 strategyId,
        bool isSwap,
        Constants.ZkProofData memory zkProof
    ) private {
        PoolId poolId = key.toId();
        IPoolManager.ModifyLiquidityParams memory modifyParams =
            abi.decode(params, (IPoolManager.ModifyLiquidityParams));
        Constants.Strategy storage strategy = strategies[poolId][strategyId];

        uint256[] memory liquidityAmounts = _calculateLiquidityAmounts(strategy, uint256(modifyParams.liquidityDelta));
        (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        for (uint256 i = 0; i < strategy.chainIds.length; i++) {
            uint256 liquidity = liquidityAmounts[i];
            uint16 destinationChainId = uint16(strategy.chainIds[i]);
            address destinationHook = strategy.hooks[i];

            BalanceDelta delta;
            if (isSwap) {
                delta = _executeSwap(
                    key, modifyParams, destinationChainId, destinationHook, liquidity, sqrtPriceX96, abi.encode(zkProof)
                );
            } else {
                delta = _executeAddLiquidity(
                    key, modifyParams, destinationChainId, destinationHook, liquidity, sqrtPriceX96, zkProof
                );
            }

            _takeDeltas(sender, key, delta);
        }
    }

    function _calculateSwapAmounts(IPoolManager.SwapParams memory params)
        private
        pure
        returns (uint256 amount0, uint256 amount1)
    {
        amount0 = params.zeroForOne ? uint256(-params.amountSpecified) : 0;
        amount1 = params.zeroForOne ? 0 : uint256(-params.amountSpecified);
    }

    function _refundRemainingTokens(Constants.CrossChainParams memory params) private {
        if (params.amount0 > 0) {
            IERC20Minimal(params.token0).transfer(params.sender, params.amount0);
        }

        if (params.amount1 > 0) {
            IERC20Minimal(params.token1).transfer(params.sender, params.amount1);
        }
    }

    // Function to calculate the liquidity amounts for each chain based on the selected strategy
    function _calculateLiquidityAmounts(Constants.Strategy storage strategy, uint256 liquidityAmount)
        private
        view
        returns (uint256[] memory liquidityAmounts)
    {
        uint256 totalAllocated = 0;
        liquidityAmounts = new uint256[](strategy.chainIds.length);

        for (uint256 i; i < strategy.percentages.length; ++i) {
            uint256 percentage = strategy.percentages[i];
            liquidityAmounts[i] = (liquidityAmount * percentage) / 100;

            totalAllocated += liquidityAmounts[i];
        }

        // Ensure no rounding errors
        uint256 roundingAdjustment = liquidityAmount - totalAllocated;
        if (roundingAdjustment > 0) {
            liquidityAmounts[0] += roundingAdjustment;
        }
    }

    function _takeDeltas(address sender, PoolKey memory key, BalanceDelta delta) private {
        poolManager.take(key.currency0, sender, uint256(uint128(-delta.amount0())));
        poolManager.take(key.currency1, sender, uint256(uint128(-delta.amount1())));
    }

    function _settleDeltas(address sender, PoolKey memory key, BalanceDelta delta) internal {
        _settleDelta(sender, key.currency0, uint128(-delta.amount0()));
        _settleDelta(sender, key.currency1, uint128(-delta.amount1()));
    }

    function _settleDelta(address sender, Currency currency, uint128 amount) private {
        currency.settle(poolManager, sender, amount, false);
    }

    // Function to determine if it's a cross-chain transaction
    function _isCrossChain(uint256 strategyId) private pure returns (bool) {
        return strategyId == 0;
    }

    // Function to determine if it's a swap transaction
    function _isSwap(bytes memory params) private pure returns (bool) {
        return params.length == 32;
    }
}
