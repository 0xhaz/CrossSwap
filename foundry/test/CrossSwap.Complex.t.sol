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
import {Currency, CurrencyLibrary} from "@uniswap/v4-core/src/types/Currency.sol";
import {PoolSwapTest} from "@uniswap/v4-core/src/test/PoolSwapTest.sol";
import {StateLibrary} from "@uniswap/v4-core/src/libraries/StateLibrary.sol";
import {IERC20Minimal} from "@uniswap/v4-core/src/interfaces/external/IERC20Minimal.sol";
import {IPositionManager} from "v4-periphery/src/interfaces/IPositionManager.sol";
import {EasyPosm} from "test/utils/EasyPosm.sol";
import {Fixtures} from "test/utils/Fixtures.sol";
import {CrossSwap} from "src/CrossSwap.sol";
import {MockERC20} from "solmate/src/test/utils/mocks/MockERC20.sol";

import {
    CCIPLocalSimulator, IRouterClient, BurnMintERC677Helper
} from "@chainlink/local/src/ccip/CCIPLocalSimulator.sol";

contract CrossSwapTest is Test, Fixtures {
    CCIPLocalSimulator public ccipLocalSimulator;
    BurnMintERC677Helper public ccipBnMToken;

    address public sourceRouterAddress;
    address public destinationRouterAddress;

    uint64 public destinationChainSelector;
    ////////////////////////////////////////////////////////////////////////////////////////////////

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

    // The two currencies (tokens) from the pool
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
        deployFreshManagerAndRouters();
        (token0, token1) = deployMintAndApprove2Currencies();

        deployAndApprovePosm(manager);

        address flagsSourceChain = address(uint160(Hooks.BEFORE_ADD_LIQUIDITY_FLAG) ^ (0x4444 << 144));

        address flagsDestinationChain = address(uint160(Hooks.BEFORE_ADD_LIQUIDITY_FLAG) ^ (0x8888 << 144));

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
        destinationRouterAddress = address(destinationRouter);

        bytes memory constructorArgs = abi.encode(manager, authorizedUser, sourceHookChainId, sourceRouterAddress);
        deployCodeTo("CrossSwap.sol:CrossSwap", constructorArgs, flagsSourceChain);

        hookSource = CrossSwap(flagsSourceChain);
        hookAddressSource = address(hookSource);
        require(hookAddressSource == flagsSourceChain, "Hook address not set correctly");

        key = PoolKey(currency0, currency1, 3000, 60, IHooks(hookSource));
        poolId = key.toId();
        manager.initialize(key, SQRT_PRICE_1_1);

        deployFreshManagerAndRouters();
        bytes memory constructorArgs2 =
            abi.encode(manager, authorizedUser, destinationHookChainId, destinationRouterAddress);
        deployCodeTo("CrossSwap.sol:CrossSwap", constructorArgs2, flagsDestinationChain);
        hookDestination = CrossSwap(flagsDestinationChain);
        hookAddressDestination = address(hookDestination);
        require(hookAddressDestination == flagsDestinationChain, "Hook address not set correctly");

        key2 = PoolKey(currency0, currency1, 3000, 60, IHooks(hookDestination));
        poolId2 = key2.toId();
        manager.initialize(key2, SQRT_PRICE_1_1);

        uint256[] memory chainIds = new uint256[](2);
        chainIds[0] = sourceHookChainId;
        chainIds[1] = destinationHookChainId;
        uint256[] memory percentages = new uint256[](2);
        percentages[0] = 40;
        percentages[1] = 60;
        uint64[] memory selectors = new uint64[](2);
        selectors[0] = chainSelector;
        selectors[1] = chainSelector;

        address[] memory hooks = new address[](2);
        hooks[0] = hookAddressSource;
        hooks[1] = hookAddressDestination;
        vm.prank(authorizedUser);
        hookSource.addStrategy(poolId, 1, chainIds, percentages, selectors, hooks);
    }

    function test_AddLiquidityToCrossChainStrategy() public {
        deal(address(hookAddressSource), 1 ether);
        deal(address(hookDestination), 1 ether);
        ccipBnMToken.drip(address(hookSource));
        ccipBnMToken.drip(address(hookDestination));

        IERC20Minimal(Currency.unwrap(key.currency0)).approve(hookAddressSource, 1000 ether);
        IERC20Minimal(Currency.unwrap(key.currency1)).approve(hookAddressSource, 1000 ether);

        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);

        uint256 balance0Before = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1Before = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        console2.log("balance0Before", balance0Before);
        console2.log("balance1Before", balance1Before);

        hookSource.addLiquidityWithCrossChainStrategy(
            key, IPoolManager.ModifyLiquidityParams(tickLower, tickUpper, 10_000_000, bytes32(0)), 1
        );

        uint256 balance0AfterManager = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(manager));
        uint256 balance1AfterManager = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(manager));

        console2.log("balance0AfterManager", balance0AfterManager);
        console2.log("balance1AfterManager", balance1AfterManager);

        uint256 balance0After = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1After = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        console2.log("balance0After", balance0After);
        console2.log("balance1After", balance1After);

        // uint256 balance0AfterDestination =
        //     IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(hookAddressDestination);
        // uint256 balance1AfterDestination =
        //     IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(hookAddressDestination);

        // console2.log(unicode"🚀 Destination Hook Balance0:", balance0AfterDestination);
        // console2.log(unicode"🚀 Destination Hook Balance1:", balance1AfterDestination);

        // assertEq(balance0AfterDestination, 6_000_000);
        // assertEq(balance1AfterDestination, 6_000_000);

        assertEq(balance0Before - balance0After, 4_000_000);
        assertEq(balance1Before - balance1After, 4_000_000);
    }
}
