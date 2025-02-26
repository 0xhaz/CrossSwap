// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {PoolManager} from "@uniswap/v4-core/src/PoolManager.sol";
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
import {IUnlockCallback} from "@uniswap/v4-core/src/interfaces/callback/IUnlockCallback.sol";
import {BeforeSwapDelta, BeforeSwapDeltaLibrary} from "@uniswap/v4-core/src/types/BeforeSwapDelta.sol";
import {LiquidityAmounts} from "@uniswap/v4-core/test/utils/LiquidityAmounts.sol";
import {Constants, Errors, Events} from "src/libraries/Constants.sol";
import {IZkLightClient} from "src/interfaces/IZKLightClient.sol";
import {IZKVerifier} from "src/interfaces/IZKVerifier.sol";
import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";
import {ISharedLiquidityLedger} from "src/interfaces/ISharedLiquidityLedger.sol";
import {IMerkleTree} from "src/interfaces/IMerkleTree.sol";
import {CrossSwapCore} from "src/core/CrossSwapCore.sol";
import {console2} from "forge-std/Test.sol";

contract CrossSwap is CrossSwapCore {
    using CurrencyLibrary for Currency;
    using CurrencySettler for Currency;
    using PoolIdLibrary for PoolKey;
    using SafeCast for uint256;
    using SafeCast for uint128;
    using StateLibrary for IPoolManager;

    /*//////////////////////////////////////////////////////////////
                           STORAGE VARIABLES
    //////////////////////////////////////////////////////////////*/

    // Mapping of strategy IDs to their respective liquidity distribution strategies
    mapping(PoolId => mapping(uint256 => Constants.Strategy)) internal strategies;
    // Mapping to keep track of the details of the received messages
    mapping(bytes32 => Constants.CrossChainParams) public messageDetail;

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTOR
    //////////////////////////////////////////////////////////////*/

    /// @notice Constructor initializes the contract with the address of the router
    constructor(
        IPoolManager poolManager,
        address authorizedUser,
        address zkClient_,
        address sharedLiquidityLedger_,
        uint256 hookChainId
    ) CrossSwapCore(poolManager, authorizedUser, zkClient_, sharedLiquidityLedger_, hookChainId) {}

    /*//////////////////////////////////////////////////////////////
                            ADMIN FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    /// @notice Function to set the authorized user
    /// @param authorizedUser The address of the authorized user
    function setAuthorizedUser(address authorizedUser) external onlyAuthorizedUser {
        authorizedUser_ = authorizedUser;
    }

    /// @notice Function to set the hook's chain ID
    /// @param hookChainId The chain ID of the hook
    function setHookChainId(uint256 hookChainId) external onlyAuthorizedUser {
        hookChainId_ = hookChainId;
    }

    /*//////////////////////////////////////////////////////////////
                             HOOK FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    /// @notice Hook that is called before adding liquidity to a pool
    function beforeAddLiquidity(
        address sender,
        PoolKey calldata key,
        IPoolManager.ModifyLiquidityParams calldata params,
        bytes calldata data
    ) external override returns (bytes4) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));
        // Verify zk-SNARK proof
        _verifyProof(zkProof, false);

        PoolId poolId = key.toId();
        (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        // Check if this liquidity position already exists
        bytes32 existingRoot = getLatestLiquidityState(hookChainId_);

        if (existingRoot == bytes32(zkProof.publicSignals[0])) {
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
                zkProof: zkProof,
                isSwap: false,
                isCrossChain: false
            })
        );

        return this.beforeAddLiquidity.selector;
    }

    /// @notice Hook that is called after adding liquidity to a pool
    function afterAddLiquidity(
        address sender,
        PoolKey calldata,
        IPoolManager.ModifyLiquidityParams calldata,
        BalanceDelta delta,
        BalanceDelta,
        bytes calldata data
    ) external override returns (bytes4, BalanceDelta) {
        // Ensure only the contract can call this function
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        // Decode the ZK proof data
        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));
        // Verify zk-SNARK proof
        _verifyProof(zkProof, false);

        // Fetch the latest state root
        bytes32 expectedMerkleRoot = getMerkleRoot();

        // Fetch the latest state root from the ledger
        bytes32 latestStateRoot = getLatestLiquidityState(hookChainId_);
        require(latestStateRoot == expectedMerkleRoot, "CrossSwap: Invalid state root");

        // Fetch Merkle proof the the last inserted state root
        uint256 proofIndex = currentIndex() - 1;
        bytes32[TREE_DEPTH] memory proof = getMerkleProof(proofIndex);

        // Verify the Merkle proof
        require(verifyProof(expectedMerkleRoot, proof, latestStateRoot, proofIndex), "CrossSwap: Invalid Merkle proof");

        updateLiquidityState(hookChainId_, expectedMerkleRoot, abi.encode(zkProof));

        emit Events.MerkleRootValidated(expectedMerkleRoot);

        return (this.afterAddLiquidity.selector, delta);
    }

    /// @notice Hook that is called before removing liquidity from a pool
    function beforeRemoveLiquidity(
        address sender,
        PoolKey calldata key,
        IPoolManager.ModifyLiquidityParams calldata params,
        bytes calldata data
    ) external override returns (bytes4) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        // Decode the ZK proof data
        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));

        // Construct CrossChainMessage
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
            zkProof: data
        });

        // Process liquidity update
        _processLiquidity(receivedMessage, abi.encode(zkProof));

        return this.beforeRemoveLiquidity.selector;
    }

    /// @notice Hook that is called after removing liquidity from a pool
    function afterRemoveLiquidity(
        address sender,
        PoolKey calldata,
        IPoolManager.ModifyLiquidityParams calldata,
        BalanceDelta delta,
        BalanceDelta,
        bytes calldata data
    ) external override returns (bytes4, BalanceDelta) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        // Decode the ZK proof data
        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));

        // Verify zk-SNARK proof
        _verifyProof(zkProof, false);

        // Fetch the latest state root
        bytes32 latestStateRoot = getLatestLiquidityState(hookChainId_);

        // Fetch the expected Merkle root
        bytes32 expectedMerkleRoot = getMerkleRoot();
        require(latestStateRoot == expectedMerkleRoot, "CrossSwap: Invalid state root");

        // Fetch Merkle proof the the last inserted state root
        uint256 proofIndex = currentIndex() - 1;
        bytes32[TREE_DEPTH] memory proof = getMerkleProof(proofIndex);

        // Verify the Merkle proof
        require(verifyProof(expectedMerkleRoot, proof, latestStateRoot, proofIndex), "CrossSwap: Invalid Merkle proof");

        updateLiquidityState(hookChainId_, expectedMerkleRoot, abi.encode(zkProof));

        emit Events.MerkleRootValidated(expectedMerkleRoot);

        return (this.afterRemoveLiquidity.selector, delta);
    }

    /// @notice Hook that is called before swapping tokens in a pool
    function beforeSwap(
        address sender,
        PoolKey calldata key,
        IPoolManager.SwapParams calldata params,
        bytes calldata data
    ) external override returns (bytes4, BeforeSwapDelta, uint24) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        // Decode the ZK proof data
        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));

        // Verify zk-SNARK proof
        _verifyProof(zkProof, true);

        Constants.LiquidityParams memory swapParams = Constants.LiquidityParams({
            sender: sender,
            key: key,
            params: IPoolManager.ModifyLiquidityParams({liquidityDelta: 0, tickLower: 0, tickUpper: 0, salt: 0}),
            swapParams: params,
            destinationChainId: uint16(hookChainId_),
            destinationHook: address(this),
            liquidity: 0,
            sqrtPriceX96: 0,
            zkProof: zkProof,
            isSwap: true,
            isCrossChain: false
        });

        // Execute the swap
        _executePrivacySwap(swapParams);

        return (this.beforeSwap.selector, BeforeSwapDeltaLibrary.ZERO_DELTA, 0);
    }

    /// @notice Hook that is called after swapping tokens in a pool
    function afterSwap(address, PoolKey calldata, IPoolManager.SwapParams calldata, BalanceDelta, bytes calldata data)
        external
        override
        returns (bytes4, int128)
    {
        require(msg.sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        // Decode the ZK proof data
        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));

        // Verify zk-SNARK proof
        _verifyProof(zkProof, true);

        // Compute the expected Merkle root
        bytes32 newStateRoot = getMerkleRoot();
        require(newStateRoot != bytes32(0), "CrossSwap: Invalid state root");

        // Update the liquidity state
        updateLiquidityState(hookChainId_, newStateRoot, abi.encode(zkProof));

        emit Events.MerkleRootValidated(newStateRoot);

        return (this.afterSwap.selector, 0);
    }

    /*//////////////////////////////////////////////////////////////
                           EXTERNAL FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    // Function to add a new strategy to the contract
    function addStrategy(
        PoolId poolId,
        uint256 strategyId,
        uint256[] memory chainIds,
        uint256[] memory liquidityPercentages,
        address[] memory hooks
    ) external onlyAuthorizedUser {
        // Check that the strategy ID is not already in use
        require(strategies[poolId][strategyId].chainIds.length == 0, "CrossSwap: Strategy ID already in use");

        // Check that the chain IDs and liquidity percentages arrays are of the same length
        require(
            chainIds.length == liquidityPercentages.length,
            "CrossSwap: Chain IDs and liquidity percentages arrays must be of the same length"
        );

        // Check that the liquidity percentages sum up to 100
        uint256 totalLiquidityPercentage;
        for (uint256 i; i < liquidityPercentages.length; i++) {
            totalLiquidityPercentage += liquidityPercentages[i];
        }
        require(totalLiquidityPercentage == 100, "CrossSwap: Liquidity percentages must sum up to 100");

        // Add the new strategy to the contract
        strategies[poolId][strategyId] =
            Constants.Strategy({chainIds: chainIds, percentages: liquidityPercentages, hooks: hooks});

        // Emit the StrategyAdded event
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

        strategies[poolId][strategyId] =
            Constants.Strategy({chainIds: chainIds, percentages: liquidityPercentages, hooks: hooks});

        emit Events.StrategyUpdated(poolId, strategyId);
    }

    function removeStrategy(PoolId poolId, uint256 strategyId) external onlyAuthorizedUser {
        require(strategies[poolId][strategyId].chainIds.length > 0, "CrossSwap: Strategy ID does not exist");
        delete strategies[poolId][strategyId];
        emit Events.StrategyRemoved(poolId, strategyId);
    }

    function setZkClient(address zkClientAddress) external {
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

        // Decode zkProof data
        Constants.ZkProofData memory zkProof = abi.decode(data.params, (Constants.ZkProofData));

        if (isCrossChain) {
            _handleCrossChain(data.sender, key, data.params, isSwap, zkProof, hookChainId_);
        } else {
            _handleLocalTransaction(key, data.sender, data.params, data.strategyId, isSwap, zkProof);
        }

        return abi.encode(BalanceDeltaLibrary.ZERO_DELTA);
    }

    /*//////////////////////////////////////////////////////////////
                             CROSS CHAIN FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _sendMessage(Constants.CrossChainParams memory params) internal returns (bytes32 messageId) {
        bytes memory payload = abi.encode(params);

        uint256 estimateFee = estimateFee(params.destinationChainId);
        require(msg.value >= estimateFee, "CrossSwap: Insufficient fee");

        sendMessage(params.destinationChainId, params.destinationHook, payload);

        messageId = keccak256(payload);
    }

    /// @notice Function to receive a message from another chain
    function zkReceive(uint16 srcChainId, bytes calldata payload) external {
        require(msg.sender == address(zkClient), "CrossSwap: Unauthorized sender");

        Constants.CrossChainParams memory receivedMessage = abi.decode(payload, (Constants.CrossChainParams));
        Constants.LiquidityParams memory swapParams = abi.decode(payload, (Constants.LiquidityParams));

        address token0 = Currency.unwrap(receivedMessage.key.currency0);
        address token1 = Currency.unwrap(receivedMessage.key.currency1);

        PoolKey memory key = PoolKey({
            currency0: Currency.wrap(token0),
            currency1: Currency.wrap(token1),
            fee: receivedMessage.key.fee,
            tickSpacing: receivedMessage.key.tickSpacing,
            hooks: IHooks(address(this))
        });

        Constants.ZkProofData memory zkProof = abi.decode(receivedMessage.zkProof, (Constants.ZkProofData));

        if (receivedMessage.isSwap) {
            _verifyProof(zkProof, true);

            _executePrivacySwap(
                Constants.LiquidityParams({
                    sender: receivedMessage.sender,
                    key: key,
                    params: IPoolManager.ModifyLiquidityParams({liquidityDelta: 0, tickLower: 0, tickUpper: 0, salt: 0}),
                    swapParams: swapParams.swapParams,
                    destinationChainId: srcChainId,
                    destinationHook: address(this),
                    liquidity: 0,
                    sqrtPriceX96: 0,
                    zkProof: zkProof,
                    isSwap: true,
                    isCrossChain: true
                })
            );
        } else {
            _processLiquidity(receivedMessage, abi.encode(zkProof));

            IPoolManager.ModifyLiquidityParams memory modifyParams = IPoolManager.ModifyLiquidityParams({
                liquidityDelta: int256(uint256(receivedMessage.amount0)),
                tickLower: receivedMessage.tickLower,
                tickUpper: receivedMessage.tickUpper,
                salt: bytes32(0)
            });

            BalanceDelta delta = abi.decode(
                poolManager.unlock(
                    abi.encode(
                        Constants.CallbackData({
                            sender: receivedMessage.sender,
                            key: key,
                            params: abi.encode(modifyParams),
                            strategyId: 1
                        })
                    )
                ),
                (BalanceDelta)
            );

            _settleDeltas(receivedMessage.sender, key, delta);
        }
    }

    /*//////////////////////////////////////////////////////////////
                            HELPER FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _processLiquidity(Constants.CrossChainParams memory receivedMessage, bytes memory zkProof) private {
        address token0 = Currency.unwrap(receivedMessage.key.currency0);
        address token1 = Currency.unwrap(receivedMessage.key.currency1);

        Constants.ZkProofData memory proofData = abi.decode(zkProof, (Constants.ZkProofData));

        _verifyProof(proofData, false);

        PoolKey memory key = PoolKey({
            currency0: Currency.wrap(token0),
            currency1: Currency.wrap(token1),
            fee: receivedMessage.key.fee,
            tickSpacing: receivedMessage.key.tickSpacing,
            hooks: IHooks(address(this))
        });

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
                        strategyId: 1
                    })
                )
            ),
            (BalanceDelta)
        );

        receivedMessage.amount0 -= uint256(uint128(delta.amount0()));
        receivedMessage.amount1 -= uint256(uint128(delta.amount1()));

        _refundRemainingTokens(receivedMessage);
    }

    function _refundRemainingTokens(Constants.CrossChainParams memory params) private {
        address token0 = Currency.unwrap(params.key.currency0);
        address token1 = Currency.unwrap(params.key.currency1);

        if (params.amount0 > 0) {
            IERC20Minimal(token0).transfer(params.sender, params.amount0);
        }

        if (params.amount1 > 0) {
            IERC20Minimal(token1).transfer(params.sender, params.amount1);
        }
    }

    // Function to calculate the liquidity amounts for each chain based on the selected strategy
    function _calculateLiquidityAmounts(Constants.Strategy storage strategy, uint256 liquidityAmount)
        internal
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
        uint256 remainingAdjustment;
        for (uint256 i = 0; i < strategy.percentages.length && remainingAdjustment > 0; i++) {
            uint256 adjust = remainingAdjustment / (strategy.percentages.length - i);
            liquidityAmounts[i] += adjust;
            remainingAdjustment -= adjust;
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

    function _takeDeltas(address sender, PoolKey memory key, BalanceDelta delta) internal {
        poolManager.take(key.currency0, sender, uint256(uint128(-delta.amount0())));
        poolManager.take(key.currency1, sender, uint256(uint128(-delta.amount1())));
    }

    function _settleDeltas(address sender, PoolKey memory key, BalanceDelta delta) internal {
        _settleDelta(sender, key.currency0, uint128(-delta.amount0()));
        _settleDelta(sender, key.currency1, uint128(-delta.amount1()));
    }

    function _settleDelta(address sender, Currency currency, uint128 amount) internal {
        currency.settle(poolManager, sender, amount, false);
    }

    function _executePrivacySwap(Constants.LiquidityParams memory swapParams) private {
        require(swapParams.isSwap, "CrossSwap: Not a swap");

        Constants.ZkProofData memory proofData = swapParams.zkProof;

        _verifyProof(proofData, true);

        (uint256 amount0, uint256 amount1) = _calculateSwapAmounts(swapParams.swapParams);

        BalanceDelta swapDelta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: swapParams.sender,
                        key: swapParams.key,
                        params: abi.encode(swapParams.params),
                        strategyId: 1
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
            zkProof: abi.encode(proofData)
        });

        if (swapParams.isCrossChain) {
            _transferCrossChain(crossChainParams);
        }
    }

    function _verifyProof(Constants.ZkProofData memory zkProof, bool isSwap) internal view {
        uint256 length = isSwap ? 5 : 4;
        require(zkProof.publicSignals.length == length, "CrossSwap: Invalid number of public signals");

        if (isSwap) {
            uint256[5] memory fixedSignals;
            for (uint256 i = 0; i < length; i++) {
                fixedSignals[i] = zkProof.publicSignals[i];
            }
            require(
                sharedLiquidityLedger.verifySwapProof(zkProof.proofA, zkProof.proofB, zkProof.proofC, fixedSignals),
                "CrossSwap: Invalid swap proof"
            );
        } else {
            uint256[4] memory fixedSignals;
            for (uint256 i = 0; i < length; i++) {
                fixedSignals[i] = zkProof.publicSignals[i];
            }
            require(
                sharedLiquidityLedger.verifyLiquidityProof(zkProof.proofA, zkProof.proofB, zkProof.proofC, fixedSignals),
                "CrossSwap: Invalid liquidity proof"
            );
        }
    }

    function _executeAddLiquidity(Constants.LiquidityParams memory liquidityParams)
        internal
        returns (BalanceDelta delta)
    {
        // Compute token amounts
        (uint256 amount0, uint256 amount1) =
            _calculateTokenAmounts(liquidityParams.params, liquidityParams.liquidity, liquidityParams.sqrtPriceX96);

        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: liquidityParams.sender,
                        key: liquidityParams.key,
                        params: abi.encode(liquidityParams.params),
                        strategyId: 1
                    })
                )
            ),
            (BalanceDelta)
        );

        _settleDeltas(liquidityParams.sender, liquidityParams.key, delta);

        // Update Merkle tree with new liquidity state
        bytes32 newStateRoot = PoseidonHasherLibrary.hashSingle(bytes32(amount0), bytes32(amount1));
        _updateMerkleTree(uint16(hookChainId_), newStateRoot);

        emit Events.MerkleRootUpdated(uint16(hookChainId_), newStateRoot);

        // Store proof data for state consistency
        bytes memory zkProofEncoded = abi.encode(liquidityParams.zkProof);
        updateLiquidityState(liquidityParams.destinationChainId, newStateRoot, zkProofEncoded);

        return delta;
    }

    function _updateMerkleTree(uint16 chainId, bytes32 newStateRoot) private {
        bytes32 latestStateRoot = getLatestLiquidityState(chainId);
        require(newStateRoot != latestStateRoot, "CrossSwap: state root unchanged");

        insert(newStateRoot);

        uint256 newLeafIndex = currentIndex() - 1;

        bytes32[TREE_DEPTH] memory proof = getMerkleProof(newLeafIndex);

        bytes32 merkleRoot = getMerkleRoot();
        require(verifyProof(newStateRoot, proof, merkleRoot, newLeafIndex), "CrossSwap: Invalid Merkle proof");
    }

    function _handleCrossChain(
        address sender,
        PoolKey memory key,
        bytes memory params,
        bool isSwap_,
        Constants.ZkProofData memory zkProof,
        uint256 hookChainId
    ) private {
        require(
            (isSwap_ && zkProof.publicSignals.length == 5 || !isSwap_ && zkProof.publicSignals.length == 4),
            "CrossSwap: Invalid number of public signals"
        );

        if (isSwap_) {
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
                zkProof: zkProof,
                isSwap: true,
                isCrossChain: true
            });

            _executePrivacySwap(liquidityParams);
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
                zkProof: abi.encode(zkProof)
            });

            _processLiquidity(receivedMessage, abi.encode(zkProof));
        }
    }

    function _handleLocalTransaction(
        PoolKey memory key,
        address sender,
        bytes memory params,
        uint256 strategyId,
        bool isSwap_,
        Constants.ZkProofData memory zkProof
    ) private {
        PoolId poolId = key.toId();
        IPoolManager.ModifyLiquidityParams memory modifyParams =
            abi.decode(params, (IPoolManager.ModifyLiquidityParams));
        Constants.Strategy storage strategy = strategies[poolId][strategyId];

        uint256[] memory liquidityAmounts = _calculateLiquidityAmounts(strategy, uint256(modifyParams.liquidityDelta));
        (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        for (uint256 i; i < strategy.chainIds.length; ++i) {
            uint256 liquidity = liquidityAmounts[i];
            uint16 destinationChainId = uint16(strategy.chainIds[i]);
            address destinationHook = strategy.hooks[i];

            BalanceDelta delta;
            if (isSwap_) {
                IPoolManager.SwapParams memory swapParams = abi.decode(params, (IPoolManager.SwapParams));

                Constants.LiquidityParams memory liquidityParams = Constants.LiquidityParams({
                    sender: sender,
                    key: key,
                    params: modifyParams,
                    swapParams: swapParams,
                    destinationChainId: destinationChainId,
                    destinationHook: destinationHook,
                    liquidity: liquidity,
                    sqrtPriceX96: sqrtPriceX96,
                    zkProof: zkProof,
                    isSwap: true,
                    isCrossChain: false
                });

                _executePrivacySwap(liquidityParams);
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
                    zkProof: zkProof,
                    isSwap: false,
                    isCrossChain: false
                });

                delta = _executeAddLiquidity(liquidityParams);
            }

            _takeDeltas(sender, key, delta);
        }
    }

    // Function to determine if it's a cross-chain transaction
    function _isCrossChain(uint16 destinationChainId) private pure returns (bool) {
        return destinationChainId != 0;
    }

    // Function to determine if it's a swap
    function _isSwap(bytes memory params) private pure returns (bool) {
        return params.length == 32;
    }

    function _calculateSwapAmounts(IPoolManager.SwapParams memory params)
        private
        pure
        returns (uint256 amount0, uint256 amount1)
    {
        amount0 = params.zeroForOne
            ? uint256(params.amountSpecified > 0 ? params.amountSpecified : -params.amountSpecified)
            : 0;
        amount1 = params.zeroForOne
            ? 0
            : uint256(params.amountSpecified > 0 ? params.amountSpecified : -params.amountSpecified);
    }

    function _transferCrossChain(Constants.CrossChainParams memory transferParams) private {
        IERC20Minimal(Currency.unwrap(transferParams.key.currency0)).transferFrom(
            transferParams.sender, address(this), transferParams.amount0
        );
        IERC20Minimal(Currency.unwrap(transferParams.key.currency1)).transferFrom(
            transferParams.sender, address(this), transferParams.amount1
        );

        _sendMessage(transferParams);
    }
}
