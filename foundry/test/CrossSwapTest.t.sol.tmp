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
// import {ZkLightClient} from "src/bridge/ZkLightClient.sol";
import {MockZkLightClient} from "test/mocks/MockZkLightClient.sol";
import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {Constants} from "src/libraries/Constants.sol";

contract CrossSwapTest is Test, Fixtures {
    MockZkLightClient public zkLightClient;
    ZKVerifier zkVerifier;

    event CrossChainLiquidityReceived(uint16 indexed srcChainId, bytes message);

    uint16 public destinationChainSelector;

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

    Currency token0;
    Currency token1;

    address authorizedUser = address(0xFEED);
    uint16 sourceHookChainId = 1;
    uint16 destinationHookChainId = 2;

    uint256 tokenId;
    int24 tickLower;
    int24 tickUpper;

    PoolKey key2;

    function setUp() public {
        deployFreshManagerAndRouters();
        (token0, token1) = deployMintAndApprove2Currencies();

        deployAndApprovePosm(manager);

        zkVerifier = new ZKVerifier();
        zkLightClient = new MockZkLightClient();

        address flagsSourceChain =
            address(uint160(Hooks.BEFORE_ADD_LIQUIDITY_FLAG | Hooks.BEFORE_SWAP_FLAG) ^ (0x4444 << 144));

        address flagsDestinationChain =
            address(uint160(Hooks.BEFORE_ADD_LIQUIDITY_FLAG | Hooks.BEFORE_SWAP_FLAG) ^ (0x8888 << 144));

        bytes memory constructorArgs = abi.encode(manager, authorizedUser, sourceHookChainId, zkVerifier, zkLightClient);
        deployCodeTo("CrossSwap.sol:CrossSwap", constructorArgs, flagsSourceChain);

        hookSource = CrossSwap(flagsSourceChain);
        hookAddressSource = address(hookSource);
        require(hookAddressSource == flagsSourceChain, "Hook address not set correctly");

        key = PoolKey(currency0, currency1, 3000, 60, IHooks(hookSource));
        poolId = key.toId();
        manager.initialize(key, SQRT_PRICE_1_1);

        deployFreshManagerAndRouters();

        bytes memory constructorArgs2 =
            abi.encode(manager, authorizedUser, destinationHookChainId, zkVerifier, zkLightClient);
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

        address[] memory hooks = new address[](2);
        hooks[0] = hookAddressSource;
        hooks[1] = hookAddressDestination;

        vm.prank(authorizedUser);
        hookSource.addStrategy(poolId, 1, chainIds, percentages, hooks);
        hookDestination.setZkClient(address(zkLightClient));
        console2.log(unicode"✅ zkClient set to:", address(hookDestination.zkClient()));
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
        assertTrue(revertAsExpected, "Expected revert");
    }

    function test_AddLiquidityToStrategy() public {
        IERC20Minimal(Currency.unwrap(key.currency0)).approve(hookAddressSource, 1_000e18);
        IERC20Minimal(Currency.unwrap(key.currency1)).approve(hookAddressSource, 1_000e18);

        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);

        uint256 balance0Before = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1Before = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        bytes memory liquidityData = abi.encode(key, tickLower, tickUpper, 10_000_000);
        bytes memory proofData = zkVerifier.generateProof(liquidityData);

        bool isValidProof = zkVerifier.verifyProof(proofData);
        assertTrue(isValidProof, "Proof is not valid");

        hookSource.addLiquidityWithCrossChainStrategy(
            key, IPoolManager.ModifyLiquidityParams(tickLower, tickUpper, 10_000_000, bytes32(0)), 1, proofData
        );

        uint256 balance0After = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1After = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        assertApproxEqAbs(balance0Before, balance0After, 10_000_000);
        assertApproxEqAbs(balance1Before, balance1After, 10_000_000);
    }

    function test_AddLiquidityToCrossChainStrategy() public {
        address receiver = address(hookDestination);

        deal(address(hookAddressSource), 1 ether);
        deal(address(hookDestination), 1 ether);

        IERC20Minimal(Currency.unwrap(key.currency0)).approve(hookAddressSource, 1_000e18);
        IERC20Minimal(Currency.unwrap(key.currency1)).approve(hookAddressSource, 1_000e18);

        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);

        uint256 balance0Before = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1Before = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        console2.log("Balance token0 before:", balance0Before);
        console2.log("Balance token1 before:", balance1Before);

        bytes memory liquidityData = abi.encode(key, tickLower, tickUpper, 10_000_000);
        bytes memory proofData = zkVerifier.generateProof(liquidityData);

        bool isValidProof = zkVerifier.verifyProof(proofData);
        console2.log(unicode"🔍 zkVerifier.verifyProof result:", isValidProof);
        require(isValidProof, "CrossSwap: Invalid ZK proof");

        Constants.SendMessageParams memory testParams = Constants.SendMessageParams({
            destinationChainId: sourceHookChainId,
            receiver: receiver,
            sender: address(this),
            token0: address(0x1234),
            amount0: 100,
            token1: address(0x5678),
            amount1: 200,
            fee: 3000,
            tickSpacing: 60,
            tickLower: -887220,
            tickUpper: 887220,
            isSwap: false,
            zkProof: proofData
        });

        bytes memory messagePayload = abi.encode(testParams);

        console2.log("Simulating cross-chain liquidity transfer");

        zkLightClient.mockReceiveMessage(receiver, sourceHookChainId, messagePayload);

        vm.expectEmit(true, true, true, true);
        emit CrossChainLiquidityReceived(sourceHookChainId, messagePayload);

        hookSource.addLiquidityWithCrossChainStrategy(
            key, IPoolManager.ModifyLiquidityParams(tickLower, tickUpper, 10_000_000, bytes32(0)), 1, proofData
        );

        uint256 balance0After = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1After = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        console2.log("Actual deduction token0:", balance0Before - balance0After);
        console2.log("Actual deduction token1:", balance1Before - balance1After);

        console2.log("Balance token0 after:", balance0After);
        console2.log("Balance token1 after:", balance1After);

        // uint256 balance0AfterManager = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(manager));
        // uint256 balance1AfterManager = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(manager));

        assertEq(balance0Before - balance0After, 4_000_000);
        assertEq(balance1Before - balance1After, 4_000_000);

        // assertEq(balance0AfterManager, 6_000_000);
        // assertEq(balance1AfterManager, 4_000_000);
    }

    function test_ExecuteSwapWithPrivacy() public {
        IERC20Minimal(Currency.unwrap(key.currency0)).approve(hookAddressSource, 1000 ether);
        IERC20Minimal(Currency.unwrap(key.currency1)).approve(hookAddressSource, 1000 ether);

        tickLower = TickMath.minUsableTick(key.tickSpacing);
        tickUpper = TickMath.maxUsableTick(key.tickSpacing);

        uint256 balance0Before = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1Before = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        IPoolManager.SwapParams memory swapParams =
            IPoolManager.SwapParams({zeroForOne: true, amountSpecified: 10_000e18, sqrtPriceLimitX96: 0});

        bytes memory swapData = abi.encode(key, swapParams);
        bytes memory zkProof = zkVerifier.generateProof(swapData);

        bool isValidProof = zkVerifier.verifyProof(zkProof);
        assertTrue(isValidProof);

        hookSource.executeSwapWithPrivacy(key, swapParams, destinationChainSelector, hookAddressDestination);

        uint256 balance0After = IERC20Minimal(Currency.unwrap(key.currency0)).balanceOf(address(this));
        uint256 balance1After = IERC20Minimal(Currency.unwrap(key.currency1)).balanceOf(address(this));

        assertEq(balance0Before - balance0After, 10_000e18);
        assertGt(balance1After, balance1Before);

        console2.log(unicode"✅ Swap executed successfully");
    }
}
