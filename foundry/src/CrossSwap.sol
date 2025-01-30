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
import {CCIPReceiver} from "@chainlink/contracts-ccip/src/v0.8/ccip/applications/CCIPReceiver.sol";
import {Client} from "@chainlink/contracts-ccip/src/v0.8/ccip/libraries/Client.sol";
import {IRouterClient} from "@chainlink/contracts-ccip/src/v0.8/ccip/interfaces/IRouterClient.sol";
import {IRouter} from "@chainlink/contracts-ccip/src/v0.8/ccip/interfaces/IRouter.sol";
import {LiquidityAmounts} from "@uniswap/v4-core/test/utils/LiquidityAmounts.sol";
import {Constants, Errors, Events} from "src/libraries/Constants.sol";
import {GKRVerifier} from "src/zk/GKRVerifier.sol";
import {console2} from "forge-std/Test.sol";

contract CrossSwap is CCIPReceiver, BaseHook {
    using CurrencyLibrary for Currency;
    using CurrencySettler for Currency;
    using PoolIdLibrary for PoolKey;
    using SafeCast for uint256;
    using SafeCast for uint128;
    using StateLibrary for IPoolManager;

    GKRVerifier public verifier;

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
    /// @param router The address of the router contract
    constructor(
        IPoolManager poolManager,
        address authorizedUser,
        uint256 hookChainId,
        address router,
        GKRVerifier _verifier
    ) BaseHook(poolManager) CCIPReceiver(router) {
        authorizedUser_ = authorizedUser;
        hookChainId_ = hookChainId;
        verifier = _verifier;
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
            beforeSwap: false,
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

    /*//////////////////////////////////////////////////////////////
                           EXTERNAL FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function addLiquidityWithCrossChainStrategy(
        PoolKey memory key,
        IPoolManager.ModifyLiquidityParams memory params,
        uint256 strategyId,
        bytes calldata gkrProof
    ) external returns (BalanceDelta delta) {
        require(verifier.verifyProof(gkrProof), "CrossSwap: Invalid GKR proof");

        delta = abi.decode(
            poolManager.unlock(
                abi.encode(
                    Constants.CallbackData({
                        sender: msg.sender,
                        key: key,
                        params: params,
                        strategyId: strategyId,
                        isCrossChainIncoming: false
                    })
                )
            ),
            (BalanceDelta)
        );
    }

    function _unlockCallback(bytes calldata rawData) internal override returns (bytes memory) {
        Constants.CallbackData memory data = abi.decode(rawData, (Constants.CallbackData));
        PoolKey memory key = data.key;
        PoolId poolId = key.toId();
        address sender = data.sender;
        bool isCrossChainIncoming = data.isCrossChainIncoming;
        IPoolManager.ModifyLiquidityParams memory params = data.params;

        Constants.Strategy storage strategy = strategies[poolId][data.strategyId];
        BalanceDelta delta;

        if (isCrossChainIncoming) {
            return abi.encode(delta);
        }

        if (data.params.liquidityDelta < 0) {
            (delta,) = poolManager.modifyLiquidity(key, params, Constants.ZERO_BYTES);
            _takeDeltas(sender, key, delta);
        } else {
            // calculate the liquidity to be added on each chain
            uint256[] memory liquidityAmounts = _calculateLiquidityAmounts(strategy, uint256(params.liquidityDelta));

            (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

            // Add liquidity to the pool if the chain ID exists in the strategy
            for (uint256 i; i < strategy.chainIds.length; ++i) {
                if (strategy.chainIds[i] != hookChainId_) {
                    (uint256 amount0, uint256 amount1) =
                        _calculateTokenAmounts(params, liquidityAmounts[i], sqrtPriceX96);

                    params.liquidityDelta -= int256(uint256(liquidityAmounts[i]));
                    _transferCrossChain(
                        sender,
                        strategy.hooks[i],
                        strategy.chainSelectors[i],
                        key,
                        amount0,
                        amount1,
                        params.tickLower,
                        params.tickUpper
                    );
                }
            }

            if (params.liquidityDelta > 0) {
                (delta,) = poolManager.modifyLiquidity(key, params, Constants.ZERO_BYTES);
                _settleDeltas(sender, key, delta);
            }
        }

        return abi.encode(delta);
    }

    /*//////////////////////////////////////////////////////////////
                             CCIP FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function sendMessage(
        uint64 destinationChainSelector,
        address sender,
        address receiver,
        address token0,
        uint256 amount0,
        address token1,
        uint256 amount1,
        uint24 fee,
        int24 tickSpacing,
        int24 tickLower,
        int24 tickUpper
    ) external returns (bytes32 messageId) {
        Constants.SendMessageParams memory params = Constants.SendMessageParams({
            destinationChainSelector: destinationChainSelector,
            receiver: receiver,
            sender: sender,
            token0: token0,
            amount0: amount0,
            token1: token1,
            amount1: amount1,
            fee: fee,
            tickSpacing: tickSpacing,
            tickLower: tickLower,
            tickUpper: tickUpper
        });

        return _sendMessage(params);
    }

    function _sendMessage(Constants.SendMessageParams memory params) internal returns (bytes32 messageId) {
        bytes memory encodeMessage =
            abi.encode(params.sender, params.fee, params.tickSpacing, params.tickLower, params.tickUpper);

        // Set the token amounts
        Client.EVMTokenAmount[] memory tokenAmounts = new Client.EVMTokenAmount[](2);
        tokenAmounts[0] = Client.EVMTokenAmount({token: params.token0, amount: params.amount0});
        tokenAmounts[1] = Client.EVMTokenAmount({token: params.token1, amount: params.amount1});

        console2.log(unicode"🚀 Preparing to Send Cross-Chain Message!");
        console2.log("Destination Chain:", params.destinationChainSelector);
        console2.log("Receiver Hook:", params.receiver);
        console2.log("Token0:", params.token0, "Amount:", params.amount0);
        console2.log("Token1:", params.token1, "Amount:", params.amount1);

        // Create an EVM2AnyMessage struct in memory
        Client.EVM2AnyMessage memory evm2AnyMessage = Client.EVM2AnyMessage({
            receiver: abi.encode(params.receiver),
            data: encodeMessage,
            tokenAmounts: tokenAmounts,
            extraArgs: Client._argsToBytes(Client.EVMExtraArgsV1({gasLimit: 500_000})),
            feeToken: address(0)
        });

        // Initialize a router client instance
        IRouterClient router = IRouterClient(this.getRouter());

        // Approve the Router to spend tokens on contract's behalf
        IERC20Minimal(params.token0).approve(address(router), params.amount0);
        IERC20Minimal(params.token1).approve(address(router), params.amount1);

        // Initialize the Router to send the message
        uint256 fees = router.getFee(params.destinationChainSelector, evm2AnyMessage);
        console2.log("Required CCIP Fee:", fees);

        // Reverts if this contract does not have enough tokens to pay the fee
        if (address(this).balance < fees) revert Errors.InsufficientFeeTokenAmount();

        // Send the message through the router and store the returned message ID
        messageId = router.ccipSend{value: fees}(params.destinationChainSelector, evm2AnyMessage);

        console2.log(unicode"✅ CCIP Message Sent Successfully!");
        console2.logBytes32(messageId);

        // Emit the MessageSent event
        emit Events.MessageSent(
            messageId, params.destinationChainSelector, params.receiver, tokenAmounts[0], tokenAmounts[1], fees
        );

        // Return the message ID
        return messageId;
    }

    /// @notice Function to receive a message from another chain
    function _ccipReceive(Client.Any2EVMMessage memory any2EvmMessage) internal virtual override {
        bytes32 messageId = any2EvmMessage.messageId;
        uint64 sourceChainSelector = any2EvmMessage.sourceChainSelector;
        address sender = abi.decode(any2EvmMessage.sender, (address));
        receivedMessages.push(messageId);

        Constants.CCIPReceiveParams memory params;
        (params.recipient, params.fee, params.tickSpacing, params.tickLower, params.tickUpper) =
            abi.decode(any2EvmMessage.data, (address, uint24, int24, int24, int24));

        Client.EVMTokenAmount[] memory tokenAmounts = any2EvmMessage.destTokenAmounts;

        params.token0Address = tokenAmounts[0].token;
        params.token0Amount = tokenAmounts[0].amount;
        params.token1Address = tokenAmounts[1].token;
        params.token1Amount = tokenAmounts[1].amount;

        console2.log(unicode"🔥 Receiving Cross-Chain Liquidity Message!");
        console2.log("Source Chain:", sourceChainSelector);
        console2.log("Sender:", sender);
        console2.log("Received Token0:", params.token0Address, "Amount:", params.token0Amount);
        console2.log("Received Token1:", params.token1Address, "Amount:", params.token1Amount);

        IERC20Minimal(params.token0Address).approve(address(poolManager), type(uint256).max);
        IERC20Minimal(params.token1Address).approve(address(poolManager), type(uint256).max);

        PoolKey memory key = PoolKey({
            currency0: Currency.wrap(params.token0Address),
            currency1: Currency.wrap(params.token1Address),
            fee: params.fee,
            tickSpacing: params.tickSpacing,
            hooks: IHooks(address(this))
        });

        PoolId poolId = key.toId();
        (uint160 currentSqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        console2.log(unicode"✅ Adding Liquidity on Destination Hook!");
        _processLiquidity(key, params, currentSqrtPriceX96);

        // Refund remaining tokens to recipient
        _refundRemainingTokens(params);

        //  Emit the MessageReceived event
        emit Events.MessageReceived(messageId, sourceChainSelector, sender, tokenAmounts[0], tokenAmounts[1]);

        messageDetail[messageId] = Constants.Message({
            sourceChainSelector: sourceChainSelector,
            sender: sender,
            token0: params.token0Address,
            amount0: params.token0Amount,
            token1: params.token1Address,
            amount1: params.token1Amount,
            fee: params.fee,
            tickSpacing: params.tickSpacing,
            tickLower: params.tickLower,
            tickUpper: params.tickUpper
        });
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
            uint64 sourceChainSelector,
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
            detail.sourceChainSelector,
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
            uint64 sourceChainSelector,
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
            messageId,
            detail.sourceChainSelector,
            detail.sender,
            detail.token0,
            detail.amount0,
            detail.token1,
            detail.amount1
        );
    }

    function getLastReceivedMessageDetails()
        external
        view
        returns (
            bytes32 messageId,
            uint64 sourceChainSelector,
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
            messageId,
            detail.sourceChainSelector,
            detail.sender,
            detail.token0,
            detail.amount0,
            detail.token1,
            detail.amount1
        );
    }

    /*//////////////////////////////////////////////////////////////
                            HELPER FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _processLiquidity(
        PoolKey memory key,
        Constants.CCIPReceiveParams memory params,
        uint160 currentSqrtPriceX96
    ) private {
        uint160 lowerSqrtPriceX96 = TickMath.getSqrtPriceAtTick(params.tickLower);
        uint160 upperSqrtPriceX96 = TickMath.getSqrtPriceAtTick(params.tickUpper);

        uint128 liquidity = LiquidityAmounts.getLiquidityForAmounts(
            currentSqrtPriceX96, lowerSqrtPriceX96, upperSqrtPriceX96, params.token0Amount, params.token1Amount
        );

        console2.log(unicode"🌊 Adding Liquidity to Pool on Destination Chain");
        console2.log("Liquidity:", liquidity);
        console2.log("Token0 Amount:", params.token0Amount);
        console2.log("Token1 Amount:", params.token1Amount);

        IPoolManager.ModifyLiquidityParams memory modifyParams = IPoolManager.ModifyLiquidityParams({
            liquidityDelta: int256(uint256(liquidity)),
            tickLower: params.tickLower,
            tickUpper: params.tickUpper,
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
                        isCrossChainIncoming: true
                    })
                )
            ),
            (BalanceDelta)
        );

        console2.log(unicode"✅ Liquidity Successfully Added to Destination Pool!");

        params.token0Amount -= uint256(uint128(delta.amount0()));
        params.token1Amount -= uint256(uint128(delta.amount1()));
    }

    function _refundRemainingTokens(Constants.CCIPReceiveParams memory params) private {
        if (params.token0Amount > 0) {
            IERC20Minimal(params.token0Address).transfer(params.recipient, params.token0Amount);
        }

        if (params.token1Amount > 0) {
            IERC20Minimal(params.token1Address).transfer(params.recipient, params.token1Amount);
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
        uint64 destinationChainSelector,
        PoolKey memory key,
        uint256 amount0,
        uint256 amount1,
        int24 tickLower,
        int24 tickUpper
    ) internal {
        console2.log(unicode"🚀 Sending Cross-Chain Liquidity!");
        console2.log("Receiver Hook:", hook);
        console2.log("Token0:", Currency.unwrap(key.currency0), "Amount:", amount0);
        console2.log("Token1:", Currency.unwrap(key.currency1), "Amount:", amount1);

        IERC20Minimal(Currency.unwrap(key.currency0)).transferFrom(sender, address(this), amount0);
        IERC20Minimal(Currency.unwrap(key.currency1)).transferFrom(sender, address(this), amount1);

        Constants.SendMessageParams memory params = Constants.SendMessageParams({
            destinationChainSelector: uint64(destinationChainSelector),
            receiver: hook,
            sender: sender,
            token0: Currency.unwrap(key.currency0),
            amount0: amount0,
            token1: Currency.unwrap(key.currency1),
            amount1: amount1,
            fee: key.fee,
            tickSpacing: key.tickSpacing,
            tickLower: tickLower,
            tickUpper: tickUpper
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
        uint64[] memory chainSelectors,
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
        strategies[poolId][strategyId] = Constants.Strategy({
            chainIds: chainIds,
            percentages: liquidityPercentages,
            chainSelectors: chainSelectors,
            hooks: hooks
        });

        // Emit the StrategyAdded event
        emit Events.StrategyAdded(poolId, strategyId, chainIds, liquidityPercentages, hooks);
    }

    function updateStrategy(
        PoolId poolId,
        uint256 strategyId,
        uint256[] memory chainIds,
        uint256[] memory liquidityPercentages,
        uint64[] memory chainSelectors,
        address[] memory hooks
    ) external onlyAuthorizedUser {
        require(strategies[poolId][strategyId].chainIds.length > 0, "CrossSwap: Strategy ID does not exist");

        strategies[poolId][strategyId] = Constants.Strategy({
            chainIds: chainIds,
            percentages: liquidityPercentages,
            chainSelectors: chainSelectors,
            hooks: hooks
        });

        emit Events.StrategyUpdated(poolId, strategyId);
    }

    function removeStrategy(PoolId poolId, uint256 strategyId) external onlyAuthorizedUser {
        require(strategies[poolId][strategyId].chainIds.length > 0, "CrossSwap: Strategy ID does not exist");
        delete strategies[poolId][strategyId];
        emit Events.StrategyRemoved(poolId, strategyId);
    }
}
