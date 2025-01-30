// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {Hooks} from "@uniswap/v4-core/src/libraries/Hooks.sol";
import {TickMath} from "@uniswap/v4-core/src/libraries/TickMath.sol";
import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
import {PoolKey} from "@uniswap/v4-core/src/types/PoolKey.sol";
import {BalanceDelta} from "@uniswap/v4-core/src/types/BalanceDelta.sol";
import {PoolId, PoolIdLibrary} from "@uniswap/v4-core/src/types/PoolId.sol";
import {CurrencyLibrary, Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {PoolSwapTest} from "@uniswap/v4-core/src/test/PoolSwapTest.sol";
import {CrossSwap} from "src/CrossSwap.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {IERC20Minimal} from "@uniswap/v4-core/src/interfaces/external/IERC20Minimal.sol";
import {IPositionManager} from "v4-periphery/src/interfaces/IPositionManager.sol";
import {EasyPosm} from "test/utils/EasyPosm.sol";
import {Fixtures} from "test/utils/Fixtures.sol";
import {MockERC20} from "solmate/src/test/utils/mocks/MockERC20.sol";
import {GKRVerifier} from "src/zk/GKRVerifier.sol";

import {
    CCIPLocalSimulator, IRouterClient, BurnMintERC677Helper
} from "@chainlink/local/src/ccip/CCIPLocalSimulator.sol";

contract CrossChainFunctionalityTest is Test, Fixtures {
    CCIPLocalSimulator public ccipLocalSimulator;
    BurnMintERC677Helper public ccipBnMToken;

    address public sourceRouterAddress;
    address public destinatioRouterAddress;
    GKRVerifier verifier;
    uint64 public destinationChainSelector;

    using EasyPosm for IPositionManager;
    using PoolIdLibrary for PoolKey;
    using CurrencyLibrary for Currency;
    using StateLibrary for IPoolManager;

    CrossSwap hookSource;
    CrossSwap hookDestination;
    address hookAddressSource;
    address hookAddressDestination;

    PoolId poolId;
    PoolId poolId2;

    // The two currencies from the pool
    Currency token0;
    Currency token1;

    address authorizedUser = address(0xFEED);
    uint256 sourceHookChainId = 1;
    uint256 destinationHookChainId = 2;

    uint256 tokenId;
    int24 tickLower;
    int24 tickUpper;

    PoolKey key2;

    function setUp() public {
        // creates the pool manager, utility routers, and test tokens
        deployFreshManagerAndRouters();
        (token0, token1) = deployMintAndApprove2Currencies();

        deployAndApprovePosm(manager);

        verifier = new GKRVerifier();

        // Deploy the hook to an address with the correct flags
        address flagSourceChain = address(
            uint160(Hooks.BEFORE_ADD_LIQUIDITY_FLAG) ^ (0x4444 << 144) // Namespace the hook to avoid collisions
        );

        address flagDestinationChain = address(
            uint160(Hooks.BEFORE_ADD_LIQUIDITY_FLAG) ^ (0x8888 << 144) // Namespace the hook to avoid collisions
        );

        ccipLocalSimulator = new CCIPLocalSimulator();

        (
            uint64 chainSelector,
            IRouterClient sourceRouter,
            IRouterClient destinationRouter,
            ,
            ,
            BurnMintERC677Helper ccipBnM,
        ) = ccipLocalSimulator.configuration();

        destinationChainSelector = chainSelector;
        ccipBnMToken = ccipBnM;

        sourceRouterAddress = address(sourceRouter);
        destinatioRouterAddress = address(destinationRouter);

        bytes memory constructorArgs =
            abi.encode(manager, authorizedUser, sourceHookChainId, sourceRouterAddress, verifier);
        bytes memory constructorArgs2 =
            abi.encode(manager, authorizedUser, destinationHookChainId, destinatioRouterAddress, verifier);
        deployCodeTo("CrossSwap.sol:CrossSwap", constructorArgs, flagSourceChain);
        deployCodeTo("CrossSwap.sol:CrossSwap", constructorArgs2, flagDestinationChain);

        hookSource = CrossSwap(flagSourceChain);
        hookDestination = CrossSwap(flagDestinationChain);

        hookAddressSource = address(hookSource);
        hookAddressDestination = address(hookDestination);
        require(hookAddressSource == flagSourceChain, "Hook address does not match the expected flag");
        require(hookAddressDestination == flagDestinationChain, "Hook address does not match the expected flag");

        // Create the pool
        key = PoolKey(currency0, currency1, 3000, 60, IHooks(hookSource));
        poolId = key.toId();
        manager.initialize(key, SQRT_PRICE_1_1);
    }

    function test_TokensLeaveSourceChain() external {
        deal(address(hookAddressSource), 1 ether);
        deal(address(hookDestination), 1 ether);
        ccipBnMToken.drip(address(hookSource));
        ccipBnMToken.drip(address(hookDestination));

        // Transfer tokens to the hook address
        token0.transfer(address(hookAddressSource), 1000e18);
        token1.transfer(address(hookAddressSource), 1000e18);

        // Approve our hook address to spend these tokens as needed
        MockERC20(Currency.unwrap(token0)).approve(address(hookSource), type(uint256).max);
        MockERC20(Currency.unwrap(token1)).approve(address(hookSource), type(uint256).max);

        vm.prank(destinatioRouterAddress);
        MockERC20(Currency.unwrap(token0)).approve(address(hookDestination), type(uint256).max);
        vm.prank(destinatioRouterAddress);
        MockERC20(Currency.unwrap(token1)).approve(address(hookDestination), type(uint256).max);

        uint256 amount0ToSend = 100;
        uint256 amount1ToSend = 500;

        uint256 token0BalanceOfSenderBefore = token0.balanceOf(address(hookAddressSource));
        uint256 token1BalanceOfSenderBefore = token1.balanceOf(address(hookAddressSource));

        console2.log("Token0 balance of sender before: ", token0BalanceOfSenderBefore);
        console2.log("Token1 balance of sender before: ", token1BalanceOfSenderBefore);

        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);

        // Send the cross-chain order
        bytes32 messageId = hookSource.sendMessage(
            destinationChainSelector,
            address(this),
            address(hookDestination),
            address(Currency.unwrap(token0)),
            amount0ToSend,
            address(Currency.unwrap(token1)),
            amount1ToSend,
            key.fee,
            key.tickSpacing,
            tickLower,
            tickUpper
        );

        uint256 token0BalanceOfSenderAfter = token0.balanceOf(address(hookSource));
        uint256 token1BalanceOfSenderAfter = token1.balanceOf(address(hookSource));

        console2.log("Token0 balance of sender after: ", token0BalanceOfSenderAfter);
        console2.log("Token1 balance of sender after: ", token1BalanceOfSenderAfter);

        console2.logUint(uint256(messageId));

        assertEq(
            token0BalanceOfSenderAfter,
            token0BalanceOfSenderBefore - amount0ToSend,
            "Token0 balance of sender should be decreased by amount0ToSend"
        );
        assertEq(
            token1BalanceOfSenderAfter,
            token1BalanceOfSenderBefore - amount1ToSend,
            "Token1 balance of sender should be decreased by amount1ToSend"
        );
        assertTrue(messageId != bytes32(0), "Message ID should not be 0");
    }

    function test_TokensLeaveSenderAndReceivedByReceiverCCIP() public {
        deal(address(hookSource), 1 ether);
        deal(address(hookDestination), 1 ether);
        ccipBnMToken.drip(address(hookSource));
        ccipBnMToken.drip(address(hookDestination));

        // Transfer tokens to the hook address
        token0.transfer(address(hookSource), 1000e18);
        token1.transfer(address(hookSource), 1000e18);

        // Approve our hook address to spend these tokens as needed
        MockERC20(Currency.unwrap(token0)).approve(address(hookSource), type(uint256).max);
        MockERC20(Currency.unwrap(token1)).approve(address(hookSource), type(uint256).max);

        uint256 amount0ToSend = 100;
        uint256 amount1ToSend = 500;

        uint256 token0BalanceOfSenderBefore = token0.balanceOf(address(hookSource));
        uint256 token1BalanceOfSenderBefore = token1.balanceOf(address(hookSource));

        console2.log("Token0 balance of sender before: ", token0BalanceOfSenderBefore);
        console2.log("Token1 balance of sender before: ", token1BalanceOfSenderBefore);

        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);

        // Send the cross-chain order
        bytes32 messageId = hookSource.sendMessage(
            destinationChainSelector,
            address(this),
            address(hookDestination),
            address(Currency.unwrap(token0)),
            amount0ToSend,
            address(Currency.unwrap(token1)),
            amount1ToSend,
            key.fee,
            key.tickSpacing,
            tickLower,
            tickUpper
        );

        uint256 token0BalanceOfSenderAfter = token0.balanceOf(address(hookSource));
        uint256 token1BalanceOfSenderAfter = token1.balanceOf(address(hookSource));

        console2.log("Token0 balance of sender after: ", token0BalanceOfSenderAfter);
        console2.log("Token1 balance of sender after: ", token1BalanceOfSenderAfter);

        console2.logUint(uint256(messageId));

        assertEq(
            token0BalanceOfSenderAfter,
            token0BalanceOfSenderBefore - amount0ToSend,
            "Token0 balance of sender should be decreased by amount0ToSend"
        );

        assertEq(
            token1BalanceOfSenderAfter,
            token1BalanceOfSenderBefore - amount1ToSend,
            "Token1 balance of sender should be decreased by amount1ToSend"
        );

        assertTrue(messageId != bytes32(0), "Message ID should not be 0");

        // Check if the message was actually sent through the CCIP router
        (
            uint64 sourceChainSelector,
            address sender,
            address token0MsgAddr,
            uint256 amount0,
            address token1MsgAddr,
            uint256 amount1,
            ,
            ,
            ,
        ) = hookDestination.getReceivedMessageDetails(messageId);

        assertEq(
            sourceChainSelector,
            destinationChainSelector,
            "Source chain selector should be the destination chain selector"
        );
        assertEq(sender, address(hookSource), "Sender should be the hook source");
        assertEq(address(Currency.unwrap(token0)), address(token0MsgAddr), "Token0 address should be the same");
        assertEq(amount0, amount0ToSend, "Amount0 should be the same");
        assertEq(address(Currency.unwrap(token1)), address(token1MsgAddr), "Token1 address should be the same");
        assertEq(amount1, amount1ToSend, "Amount1 should be the same");
    }
}
