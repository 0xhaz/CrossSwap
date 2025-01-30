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
import {GKRVerifier} from "src/zk/GKRVerifier.sol";

import {
    CCIPLocalSimulator, IRouterClient, BurnMintERC677Helper
} from "@chainlink/local/src/ccip/CCIPLocalSimulator.sol";

contract CrossSwapTest is Test, Fixtures {
    CCIPLocalSimulator public ccipLocalSimulator;
    uint64 public destinationChainSelector;
    BurnMintERC677Helper public ccipBnMToken;
    CrossSwap hookReceiver;
    address hookReceiverAddress;
    GKRVerifier verifier;

    using EasyPosm for IPositionManager;
    using CurrencyLibrary for Currency;
    using PoolIdLibrary for PoolKey;
    using StateLibrary for IPoolManager;

    CrossSwap hook;
    address hookAddress;
    PoolId poolId;

    // The two currencies from the pool
    Currency token0;
    Currency token1;

    address authorizedUser = address(0xFEED);
    uint256 originalHookChainId = 1;
    uint256 crossChainHookChainId = 2;

    uint256 tokenId;
    int24 tickLower;
    int24 tickUpper;

    function setUp() public {
        // creates the pool manager, utility routers, and test tokens
        deployFreshManagerAndRouters();
        deployMintAndApprove2Currencies();

        deployAndApprovePosm(manager);

        verifier = new GKRVerifier();

        address flags = address(uint160(Hooks.BEFORE_ADD_LIQUIDITY_FLAG) ^ (0x4444 << 144));

        bytes memory constructorArgs =
            abi.encode(manager, authorizedUser, originalHookChainId, address(0xBEEF), verifier);
        deployCodeTo("CrossSwap.sol:CrossSwap", constructorArgs, flags);
        hook = CrossSwap(flags);
        hookAddress = address(hook);
        console2.logAddress(hookAddress);
        require(hookAddress == flags, "Hook address not set correctly");
        uint256[] memory chainIds = new uint256[](1);
        chainIds[0] = originalHookChainId;
        uint256[] memory percentages = new uint256[](1);
        percentages[0] = 100;
        uint64[] memory selectors = new uint64[](1);
        selectors[0] = 1;
        address[] memory hooks = new address[](1);
        hooks[0] = address(0xdd0d);

        key = PoolKey(currency0, currency1, 3000, 60, IHooks(hook));
        poolId = key.toId();
        manager.initialize(key, SQRT_PRICE_1_1);
        vm.prank(authorizedUser);
        hook.addStrategy(poolId, 1, chainIds, percentages, selectors, hooks);
    }

    function test_CantAddLiquidity() public {
        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);
        bytes4 mintSelector =
            bytes4(keccak256("mint(PoolKey,int24,int24,uint256,uint256,uint256,address,uint256,bytes)"));

        bytes memory _calldata = abi.encodeWithSelector(
            mintSelector,
            key,
            tickLower,
            tickUpper,
            10_000e18,
            MAX_SLIPPAGE_ADD_LIQUIDITY,
            MAX_SLIPPAGE_ADD_LIQUIDITY,
            address(this),
            block.timestamp,
            ZERO_BYTES
        );
        vm.expectRevert(bytes(""));
        (bool revertAsExpected,) = address(posm).call(_calldata);
        assertTrue(revertAsExpected, "Should have reverted");
    }

    function test_GKRCompressionVerification() public {}

    function test_AddLiquidityToStrategy() public {
        // Provide full-range liquidity to the pool
        // Add some initial liquidity through the custom `addLiquidity` function
        IERC20Minimal(Currency.unwrap(key.currency0)).approve(hookAddress, 1000 ether);
        IERC20Minimal(Currency.unwrap(key.currency1)).approve(hookAddress, 1000 ether);

        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);

        uint256 balance0Before = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1Before = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        bytes memory liquidityData = abi.encode(key, tickLower, tickUpper, 10_000_000);
        bytes memory proofData = verifier.generateProof(liquidityData);

        bool isValidProof = verifier.verifyProof(proofData);
        assertTrue(isValidProof, "Proof is not valid");

        hook.addLiquidityWithCrossChainStrategy(
            key, IPoolManager.ModifyLiquidityParams(tickLower, tickUpper, 10_000_000, bytes32(0)), 1, proofData
        );

        uint256 balance0After = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1After = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        assertEq(balance0Before - balance0After, 10_000_000);
        assertEq(balance1Before - balance1After, 10_000_000);
    }
}
