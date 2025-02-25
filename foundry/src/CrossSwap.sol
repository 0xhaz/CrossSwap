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

    IZkLightClient public zkClient;
    ISharedLiquidityLedger public sharedLiquidityLedger;

    /*//////////////////////////////////////////////////////////////
                           STORAGE VARIABLES
    //////////////////////////////////////////////////////////////*/

    // Mappping of hook's chain ID
    uint256 public hookChainId_;

    // Mapping of strategy IDs to their respective liquidity distribution strategies
    mapping(PoolId => mapping(uint256 => Constants.Strategy)) internal strategies;
    // Mapping to keep track of the details of the received messages
    mapping(bytes32 => Constants.CrossChainParams) public messageDetail;

    /*//////////////////////////////////////////////////////////////
                              CONSTRUCTOR
    //////////////////////////////////////////////////////////////*/

    /// @notice Constructor initializes the contract with the address of the router
    constructor(uint256 hookChainId)
        BaseHook(
            Hooks.Permissions({
                beforeInitialize: false,
                afterInitialize: false,
                beforeAddLiquidity: true,
                afterAddLiquidity: true,
                beforeRemoveLiquidity: true,
                afterRemoveLiquidity: true,
                beforeSwap: true,
                afterSwap: true,
                beforeDonate: false,
                afterDonate: false,
                beforeSwapReturnDelta: false,
                afterSwapReturnDelta: false,
                afterAddLiquidityReturnDelta: false,
                afterRemoveLiquidityReturnDelta: false
            })
        )
    {
        hookChainId_ = hookChainId;
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
    ) external view override returns (bytes4) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));
        // Verify zk-SNARK proof
        _verifyProof(zkProof, false);

        PoolId poolId = key.toId();
        (uint160 sqrtPriceX96,,,) = poolManager.getSlot0(poolId);

        // Check if this liquidity position already exists
        bytes32 existingRoot = getLatestLiquidityState(hookChainId_);

        if (existingRoot == zkProof.publicSignals[0]) {
            return this.beforeAddLiquidity.selector;
        }

        _executeAddLiquidity(
            key, params, uint16(hookChainId_), address(this), uint256(params.liquidityDelta), sqrtPriceX96, zkProof
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
            token0: Currency.unwrap(key.currency0),
            amount0: uint256(params.liquidityDelta),
            token1: Currency.unwrap(key.currency1),
            amount1: 0,
            fee: key.fee,
            tickSpacing: key.tickSpacing,
            tickLower: params.tickLower,
            tickUpper: params.tickUpper,
            isSwap: false,
            zkProof: data
        });

        // Process liquidity update
        _processLiquidity(receivedMessage, zkProof);

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
    ) external view override returns (bytes4, BeforeSwapDelta, uint24) {
        require(sender == address(this), "CrossSwap: Unauthorized sender");
        require(data.length > 0, "CrossSwap: Missing ZK proof data");

        // Decode the ZK proof data
        Constants.ZkProofData memory zkProof = abi.decode(data, (Constants.ZkProofData));

        // Verify zk-SNARK proof
        _verifyProof(zkProof, true);

        // Execute the swap
        _executePrivacySwap(key, params, zkProof);

        return (this.beforeSwap.selector, BeforeSwapDeltaLibrary.ZERO_DELTA, 0);
    }

    /// @notice Hook that is called after swapping tokens in a pool
    function afterSwap(
        address sender,
        PoolKey calldata,
        IPoolManager.SwapParams calldata,
        BalanceDelta delta,
        bytes calldata data
    ) external override returns (bytes4, int128) {
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

    function _unlockCallback(bytes calldata rawData) internal override returns (bytes memory) {
        Constants.CallbackData memory data = abi.decode(rawData, (Constants.CallbackData));
        PoolKey memory key = data.key;

        return abi.encode(delta);
    }

    /*//////////////////////////////////////////////////////////////
                             CROSS CHAIN FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function sendMessage(Constants.CrossChainParams memory params) external returns (bytes32 messageId) {
        Constants.CrossChainParams memory sendMsg = Constants.CrossChainParams({
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

        return _sendMessage(sendMsg);
    }

    function _sendMessage(Constants.CrossChainParams memory params) internal returns (bytes32 messageId) {
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
        returns (Constants.CrossChainParams memory)
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

    function _processLiquidity(Constants.CrossChainParams memory receivedMessage, bytes zkProof) private {
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

    function _executePrivacySwap(
        PoolKey memory key,
        Constants.CrossChainParams memory params,
        Constants.ZkProofData memory zkProof
    ) internal {
        require(zkVerifier.verifyProof(zkProof), "CrossSwap: Invalid ZK proof");

        bytes32 latestStateRoot = sharedLiquidityLedger.getLatestLiquidityState(receivedMessage.sourceChainId);
        bytes32[TREE_DEPTH] memory proof = stateTree.getMerkleProof(receivedMessage.sourceChainId);

        // require(
        //     stateTree.verifyProof(latestStateRoot, proof.stateTree.getMerkleRoot()), "CrossSwap: Invalid state root"
        // );

        key = PoolKey({
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

    function _verifyProof(Constants.ZkProofData memory zkProof, bool isSwap) internal view {
        require(zkProof.publicSignals.length == (isSwap ? 5 : 4), "CrossSwap: Invalid number of public signals");

        uint256[] memory fixedSignals = zkProof.publicSignals;

        if (isSwap) {
            require(
                sharedLiquidityLedger.verifySwapProof(zkProof.proofA, zkProof.proofB, zkProof.proofC, fixedSignals),
                "CrossSwap: Invalid swap proof"
            );
        } else {
            require(
                sharedLiquidityLedger.verifyLiquidityProof(zkProof.proofA, zkProof.proofB, zkProof.proofC, fixedSignals),
                "CrossSwap: Invalid liquidity proof"
            );
        }
    }

    function _executeAddLiquidity(uint256 hookChainId, address sender, Constants.LiquidityParams memory liquidityParams)
        internal
        returns (BalanceDelta delta)
    {
        // Compute token amounts
        (uint256 amount0, uint256 amount1) =
            _calculateTokenAmounts(liquidityParams.params, liquidityParams.liquidity, liquidityParams.sqrtPriceX96);

        // Update Merkle tree with new liquidity state
        bytes32 newStateRoot = PoseidonHasherLibrary.hashSingle(bytes32(amount0), bytes32(amount1));
        _updateMerkleTree(hookChainId_, newStateRoot);

        emit Events.MerkleRootUpdated(hookChainId, newStateRoot);

        // Store proof data for state consistency
        bytes memory zkProofEncoded = abi.encode(liquidityParams.zkProof);
        updateLiquidityState(liquidityParams.destinationChainId, newStateRoot, zkProofEncoded);
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
        bool isSwap,
        Constants.ZkProofData memory zkProof,
        uint256 hookChainId
    ) private {
        require(
            (isSwap && zkProof.publicSignals.length == 5 || !isSwap && zkProof.publicSignals.length == 4),
            "CrossSwap: Invalid number of public signals"
        );

        Constants.CrossChainParams memory receivedMessage = Constants.CrossChainParams({
            sourceChainId: uint16(hookChainId),
            destinationChainId: 0,
            sender: sender,
            destinationHook: address(0),
            token0: Currency.unwrap(key.currency0),
            amount0: uint256(abi.encode(params, (int256))),
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
            _executePrivacySwap(key, abi.encode(params, (IPoolManager.SwapParams)), zkProof);
        } else {
            _processLiquidity(receivedMessage, zkProof);
        }
    }

    // Function to determine if it's a cross-chain transaction
    function _isCrossChain(uint16 destinationChainId) private pure returns (bool) {
        return destinationChainId != 0;
    }

    // Function to determine if it's a swap
    function isSwap(bytes memory params) private pure returns (bool) {
        return params.length == 32;
    }
}
