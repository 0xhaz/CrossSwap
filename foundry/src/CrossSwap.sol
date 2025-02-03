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
import {BalanceDelta} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
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
import {ZkLightClient} from "src/bridge/ZkLightClient.sol";
import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {console2} from "forge-std/Test.sol";

contract CrossSwap is BaseHook {
    using CurrencyLibrary for Currency;
    using CurrencySettler for Currency;
    using PoolIdLibrary for PoolKey;
    using SafeCast for uint256;
    using SafeCast for uint128;
    using StateLibrary for IPoolManager;

    ZKVerifier public zkVerifier;
    ZkLightClient public zkClient;

    /*//////////////////////////////////////////////////////////////
                           STORAGE VARIABLES
    //////////////////////////////////////////////////////////////*/

    bytes32[] public receivedMessages; // Array to keep track of the IDs of the received messages
    mapping(bytes32 => Constants.Message) public messageDetail; // Mapping to keep track of the details of the received messages

    // Authorized user
    address public authorizedUser_;

    // Mappping of hook's chain ID
    uint256 public hookChainId_;

    // Mapping of strategy IDs to their respective liquidity distribution strategies
    mapping(PoolId => mapping(uint256 => Constants.Strategy)) internal strategies;

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTOR
    //////////////////////////////////////////////////////////////*/

    /// @notice Constructor initializes the contract with the address of the router
    constructor(
        IPoolManager poolManager,
        address authorizedUser,
        uint256 hookChainId,
        ZKVerifier _zkVerifier,
        ZkLightClient _zkClient
    ) BaseHook(poolManager) {
        authorizedUser_ = authorizedUser;
        hookChainId_ = hookChainId;
        zkVerifier = _zkVerifier;
        zkClient = _zkClient;
    }

    /*//////////////////////////////////////////////////////////////
                               MODIFIERS
    //////////////////////////////////////////////////////////////*/

    /// @notice Modifier to restrict access to authorized user
    modifier onlyAuthorizedUser() {
        require(msg.sender == authorizedUser_, "CrossSwap: Unauthorized access");
        _;
    }

    /*//////////////////////////////////////////////////////////////
                                 HOOKS
    //////////////////////////////////////////////////////////////*/

    function getHookPermissions() public pure override returns (Hooks.Permissions memory) {
        return Hooks.Permissions({
            beforeInitialize: false,
            afterInitialize: false,
            beforeAddLiquidity: true,
            afterAddLiquidity: false,
            beforeRemoveLiquidity: false,
            afterRemoveLiquidity: false,
            beforeSwap: true,
            afterSwap: false,
            beforeDonate: false,
            afterDonate: false,
            beforeSwapReturnDelta: false,
            afterSwapReturnDelta: false,
            afterAddLiquidityReturnDelta: false,
            afterRemoveLiquidityReturnDelta: false
        });
    }

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
        PoolKey calldata,
        IPoolManager.ModifyLiquidityParams calldata,
        bytes calldata
    ) external view override returns (bytes4) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        return this.beforeAddLiquidity.selector;
    }

    /// @notice Hook that is called before swapping tokens in a pool
    function beforeSwap(address sender, PoolKey calldata, IPoolManager.SwapParams calldata, bytes calldata data)
        external
        view
        override
        returns (bytes4, BeforeSwapDelta, uint24)
    {
        require(sender == address(this), "CrossSwap: Unauthorized sender");

        bytes memory zkProof;
        if (data.length > 0) {
            zkProof = abi.decode(data, (bytes));
        } else {
            revert("CrossSwap: Missing ZK proof data");
        }

        require(zkVerifier.verifyProof(zkProof), "CrossSwap: Invalid ZK proof");

        return (this.beforeSwap.selector, BeforeSwapDeltaLibrary.ZERO_DELTA, 0);
    }

    /*//////////////////////////////////////////////////////////////
                           EXTERNAL FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function addLiquidityWithCrossChainStrategy(
        PoolKey memory key,
        IPoolManager.ModifyLiquidityParams memory params,
        uint256 strategyId,
        bytes calldata zkProof
    ) external returns (BalanceDelta delta) {
        require(zkVerifier.verifyProof(zkProof), "CrossSwap: Invalid GKR proof");

        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: msg.sender,
                        key: key,
                        params: params,
                        strategyId: strategyId,
                        isCrossChainIncoming: false,
                        isSwap: false,
                        swapParams: IPoolManager.SwapParams({zeroForOne: false, amountSpecified: 0, sqrtPriceLimitX96: 0}),
                        zkProof: Constants.ZERO_BYTES
                    })
                )
            ),
            (BalanceDelta)
        );
    }

    function executeSwapWithPrivacy(
        PoolKey memory key,
        IPoolManager.SwapParams memory params,
        uint16 destinationChainId,
        address destinationHook
    ) external {
        bytes memory zkProof = zkVerifier.generateProof(abi.encode(key, params));

        require(zkVerifier.verifyProof(zkProof), "CrossSwap: Invalid ZK proof");

        // transfer tokens cross-chain & execute swap
        _transferCrossChain(
            msg.sender,
            destinationHook,
            destinationChainId,
            key,
            uint256(params.amountSpecified),
            0,
            0,
            0,
            true,
            zkProof
        );
    }

    function _unlockCallback(bytes calldata rawData) internal override returns (bytes memory) {
        Constants.CallbackData memory data = abi.decode(rawData, (Constants.CallbackData));
        PoolKey memory key = data.key;
        PoolId poolId = key.toId();
        bool isCrossChainIncoming = data.isCrossChainIncoming;
        bool isSwap = data.isSwap;
        IPoolManager.ModifyLiquidityParams memory params = data.params;
        Constants.Strategy storage strategy = strategies[poolId][data.strategyId];
        BalanceDelta delta;

        if (isCrossChainIncoming) {
            if (data.isSwap) {
                _executeSwapWithPrivacy(
                    Constants.Message({
                        sourceChainId: uint16(hookChainId_),
                        sender: data.sender,
                        token0: Currency.unwrap(data.key.currency0),
                        amount0: uint256(data.swapParams.amountSpecified),
                        token1: Currency.unwrap(data.key.currency1),
                        amount1: 0,
                        fee: data.key.fee,
                        tickSpacing: data.key.tickSpacing,
                        tickLower: 0,
                        tickUpper: 0
                    }),
                    data.zkProof
                );
            } else {
                _processLiquidity(
                    Constants.Message({
                        sourceChainId: uint16(hookChainId_),
                        sender: data.sender,
                        token0: Currency.unwrap(data.key.currency0),
                        amount0: uint256(data.params.liquidityDelta),
                        token1: Currency.unwrap(data.key.currency1),
                        amount1: 0,
                        fee: data.key.fee,
                        tickSpacing: data.key.tickSpacing,
                        tickLower: 0,
                        tickUpper: 0
                    })
                );
            }
        } else {
            uint256[] memory liquidityAmounts = _calculateLiquidityAmounts(strategy, uint256(params.liquidityDelta));

            (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

            for (uint256 i; i < strategy.chainIds.length; i++) {
                uint256 liquidity = liquidityAmounts[i];
                uint16 destinationChainId = uint16(strategy.chainIds[i]);
                address destinationHook = strategy.hooks[i];

                if (isSwap) {
                    delta = _executeSwap(
                        key, params, destinationChainId, destinationHook, liquidity, sqrtPriceX96, data.zkProof
                    );
                } else {
                    delta = _executeAddLiquidity(
                        key, params, destinationChainId, destinationHook, liquidity, sqrtPriceX96, data.zkProof
                    );
                }

                _takeDeltas(data.sender, key, delta);
            }
        }

        return abi.encode(delta);
    }

    function _executeAddLiquidity(
        PoolKey memory key,
        IPoolManager.ModifyLiquidityParams memory params,
        uint16 destinationChainId,
        address destinationHook,
        uint256 liquidity,
        uint160 sqrtPriceX96,
        bytes memory zkProof
    ) internal returns (BalanceDelta delta) {
        (uint256 amount0, uint256 amount1) = _calculateTokenAmounts(params, liquidity, sqrtPriceX96);

        _transferCrossChain(
            msg.sender,
            destinationHook,
            destinationChainId,
            key,
            amount0,
            amount1,
            params.tickLower,
            params.tickUpper,
            false,
            zkProof
        );

        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: msg.sender,
                        key: key,
                        params: params,
                        strategyId: 1,
                        isCrossChainIncoming: true,
                        isSwap: false,
                        swapParams: IPoolManager.SwapParams({zeroForOne: false, amountSpecified: 0, sqrtPriceLimitX96: 0}),
                        zkProof: Constants.ZERO_BYTES
                    })
                )
            ),
            (BalanceDelta)
        );
    }

    function _executeSwap(
        PoolKey memory key,
        IPoolManager.ModifyLiquidityParams memory params,
        uint16 destinationChainId,
        address destinationHook,
        uint256 liquidity,
        uint160 sqrtPriceX96,
        bytes memory zkProof
    ) internal returns (BalanceDelta delta) {
        (uint256 amount0, uint256 amount1) = _calculateTokenAmounts(params, liquidity, sqrtPriceX96);

        _transferCrossChain(
            msg.sender,
            destinationHook,
            destinationChainId,
            key,
            amount0,
            amount1,
            params.tickLower,
            params.tickUpper,
            true,
            zkProof
        );

        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: msg.sender,
                        key: key,
                        params: params,
                        strategyId: 1,
                        isCrossChainIncoming: true,
                        isSwap: true,
                        swapParams: IPoolManager.SwapParams({zeroForOne: false, amountSpecified: 0, sqrtPriceLimitX96: 0}),
                        zkProof: Constants.ZERO_BYTES
                    })
                )
            ),
            (BalanceDelta)
        );
    }

    /*//////////////////////////////////////////////////////////////
                             CROSS CHAIN FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function sendMessage(
        uint16 destinationChainId,
        address sender,
        address receiver,
        address token0,
        uint256 amount0,
        address token1,
        uint256 amount1,
        uint24 fee,
        int24 tickSpacing,
        int24 tickLower,
        int24 tickUpper,
        bool isSwap,
        bytes calldata zkProof
    ) external returns (bytes32 messageId) {
        Constants.SendMessageParams memory params = Constants.SendMessageParams({
            destinationChainId: destinationChainId,
            receiver: receiver,
            sender: sender,
            token0: token0,
            amount0: amount0,
            token1: token1,
            amount1: amount1,
            fee: fee,
            tickSpacing: tickSpacing,
            tickLower: tickLower,
            tickUpper: tickUpper,
            isSwap: isSwap,
            zkProof: zkProof
        });

        return _sendMessage(params);
    }

    function _sendMessage(Constants.SendMessageParams memory params) internal returns (bytes32 messageId) {
        bytes memory payload = abi.encode(
            params.sender,
            params.token0,
            params.amount0,
            params.token1,
            params.amount1,
            params.fee,
            params.tickSpacing,
            params.tickLower,
            params.tickUpper,
            params.isSwap,
            params.zkProof
        );

        zkClient.send(params.destinationChainId, abi.encodePacked(params.receiver), uint64(block.timestamp), payload);

        messageId = keccak256(payload);
    }

    /// @notice Function to receive a message from another chain
    function zkReceive(uint16 srcChainId, bytes memory payload) external {
        require(msg.sender == address(zkClient), "CrossSwap: Unauthorized sender");

        Constants.SendMessageParams memory params = abi.decode(payload, (Constants.SendMessageParams));

        Constants.Message memory receivedMessage = Constants.Message({
            sourceChainId: srcChainId,
            sender: params.sender,
            token0: params.token0,
            amount0: params.amount0,
            token1: params.token1,
            amount1: params.amount1,
            fee: params.fee,
            tickSpacing: params.tickSpacing,
            tickLower: params.tickLower,
            tickUpper: params.tickUpper
        });

        require(zkVerifier.verifyProof(params.zkProof), "CrossSwap: Invalid ZK proof");

        if (params.isSwap) {
            _executeSwapWithPrivacy(receivedMessage, params.zkProof);
        } else {
            _processLiquidity(receivedMessage);
        }

        emit Events.MessageReceived(
            payload, srcChainId, params.sender, params.token0, params.amount0, params.token1, params.amount1
        );
    }

    function safeDecodeSendMessageParams(bytes memory payload)
        external
        pure
        returns (Constants.SendMessageParams memory)
    {
        return abi.decode(payload, (Constants.SendMessageParams));
    }

    /// @notice Get the total number of received messages
    /// @return number The total number of received messages
    function getNumberOfReceivedMessages() external view returns (uint256 number) {
        return receivedMessages.length;
    }

    function getReceivedMessageDetails(bytes32 messageId)
        external
        view
        returns (
            uint16 sourceChainId,
            address sender,
            address token0,
            uint256 amount0,
            address token1,
            uint256 amount1,
            uint24 fee,
            int24 tickSpacing,
            int24 tickLower,
            int24 tickUpper
        )
    {
        Constants.Message memory detail = messageDetail[messageId];
        if (detail.sender == address(0)) revert Errors.MessageIdNotExists(messageId);
        return (
            detail.sourceChainId,
            detail.sender,
            detail.token0,
            detail.amount0,
            detail.token1,
            detail.amount1,
            detail.fee,
            detail.tickSpacing,
            detail.tickLower,
            detail.tickUpper
        );
    }

    function getReceivedMessageAt(uint256 index)
        external
        view
        returns (
            bytes32 messageId,
            uint16 sourceChainId,
            address sender,
            address token0,
            uint256 amount0,
            address token1,
            uint256 amount1
        )
    {
        if (index >= receivedMessages.length) revert Errors.IndexOutOfBounds(index, receivedMessages.length - 1);

        messageId = receivedMessages[index];
        Constants.Message memory detail = messageDetail[messageId];
        return (
            messageId, detail.sourceChainId, detail.sender, detail.token0, detail.amount0, detail.token1, detail.amount1
        );
    }

    function getLastReceivedMessageDetails()
        external
        view
        returns (
            bytes32 messageId,
            uint16 sourceChainId,
            address sender,
            address token0,
            uint256 amount0,
            address token1,
            uint256 amount1
        )
    {
        if (receivedMessages.length == 0) revert Errors.NoMessageReceived();

        // Fetch the last received message ID
        messageId = receivedMessages[receivedMessages.length - 1];

        // Fetch the details of the last received message
        Constants.Message memory detail = messageDetail[messageId];

        return (
            messageId, detail.sourceChainId, detail.sender, detail.token0, detail.amount0, detail.token1, detail.amount1
        );
    }

    /*//////////////////////////////////////////////////////////////
                            HELPER FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _processLiquidity(Constants.Message memory receivedMessage) private {
        PoolKey memory key = PoolKey({
            currency0: Currency.wrap(receivedMessage.token0),
            currency1: Currency.wrap(receivedMessage.token1),
            fee: receivedMessage.fee,
            tickSpacing: receivedMessage.tickSpacing,
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
                        params: modifyParams,
                        strategyId: 1,
                        isCrossChainIncoming: true,
                        isSwap: false,
                        swapParams: IPoolManager.SwapParams({zeroForOne: false, amountSpecified: 0, sqrtPriceLimitX96: 0}),
                        zkProof: Constants.ZERO_BYTES
                    })
                )
            ),
            (BalanceDelta)
        );

        receivedMessage.amount0 -= uint256(uint128(delta.amount0()));
        receivedMessage.amount1 -= uint256(uint128(delta.amount1()));

        _refundRemainingTokens(receivedMessage);
    }

    function _refundRemainingTokens(Constants.Message memory params) private {
        if (params.amount0 > 0) {
            IERC20Minimal(params.token0).transfer(params.sender, params.amount0);
        }

        if (params.amount1 > 0) {
            IERC20Minimal(params.token1).transfer(params.sender, params.amount1);
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
        uint256 roundingAdjustment = liquidityAmount - totalAllocated;
        if (roundingAdjustment > 0) {
            liquidityAmounts[0] += roundingAdjustment;
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

    function _transferCrossChain(
        address sender,
        address hook,
        uint16 destinationChainId,
        PoolKey memory key,
        uint256 amount0,
        uint256 amount1,
        int24 tickLower,
        int24 tickUpper,
        bool isSwap,
        bytes memory zkProof
    ) internal {
        if (zkProof.length == 0) {
            zkProof = zkVerifier.generateProof(abi.encode(key, amount0, amount1, isSwap));
        }

        IERC20Minimal(Currency.unwrap(key.currency0)).transferFrom(sender, address(this), amount0);
        IERC20Minimal(Currency.unwrap(key.currency1)).transferFrom(sender, address(this), amount1);

        Constants.SendMessageParams memory params = Constants.SendMessageParams({
            destinationChainId: destinationChainId,
            receiver: hook,
            sender: sender,
            token0: Currency.unwrap(key.currency0),
            amount0: amount0,
            token1: Currency.unwrap(key.currency1),
            amount1: amount1,
            fee: key.fee,
            tickSpacing: key.tickSpacing,
            tickLower: tickLower,
            tickUpper: tickUpper,
            isSwap: isSwap,
            zkProof: zkProof
        });

        _sendMessage(params);
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

    function _executeSwapWithPrivacy(Constants.Message memory receivedMessage, bytes memory zkProof) internal {
        require(zkVerifier.verifyProof(zkProof), "CrossSwap: Invalid ZK proof");

        PoolKey memory key = PoolKey({
            currency0: Currency.wrap(receivedMessage.token0),
            currency1: Currency.wrap(receivedMessage.token1),
            fee: receivedMessage.fee,
            tickSpacing: receivedMessage.tickSpacing,
            hooks: IHooks(address(this))
        });

        IPoolManager.SwapParams memory swapParams = IPoolManager.SwapParams({
            zeroForOne: receivedMessage.amount0 > 0,
            amountSpecified: int256(receivedMessage.amount0 > 0 ? receivedMessage.amount0 : receivedMessage.amount1),
            sqrtPriceLimitX96: 0
        });

        BalanceDelta swapDelta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: msg.sender,
                        key: key,
                        params: IPoolManager.ModifyLiquidityParams(0, 0, 0, bytes32(0)),
                        strategyId: 0,
                        isCrossChainIncoming: true,
                        isSwap: true,
                        swapParams: swapParams,
                        zkProof: zkProof
                    })
                )
            ),
            (BalanceDelta)
        );

        _settleDeltas(msg.sender, key, swapDelta);
    }

    function setZkClient(address zkClientAddress) external {
        zkClient = ZkLightClient(zkClientAddress);
    }

    function getZkClient() external view returns (address) {
        return address(zkClient);
    }
}
