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
import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {PoseidonHasherLibrary} from "src/libraries/PoseidonHasherLib.sol";
import {ISharedLiquidityLedger} from "src/interfaces/ISharedLiquidityLedger.sol";
import {IMerkleTree} from "src/interfaces/IMerkleTree.sol";
import {CCLib} from "src/libraries/CCLib.sol";
import {console2} from "forge-std/Test.sol";

abstract contract CrossSwapCore is BaseHook, IZkLightClient, ISharedLiquidityLedger {
    using CurrencyLibrary for Currency;
    using CurrencySettle for Currency;
    using PoolIdLibrary for PoolKey;
    using SafeCast for uint256;
    using SafeCast for uint128;
    using StateLibrary for IPoolManager;

    IZkLightClient public zkClient;
    ISharedLiquidityLedger public sharedLiquidityLedger;

    /*//////////////////////////////////////////////////////////////
                           STORAGE VARIABLES
    //////////////////////////////////////////////////////////////*/

    uint256 public constant TREE_DEPTH = 32;

    // mapping(bytes32 => Constants.Message) public messageDetail; // Mapping to keep track of the details of the received messages

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
        address _zkClient,
        address _sharedLiquidityLedger,
        Hooks.Permissions memory permissions
    ) BaseHook(poolManager) {
        authorizedUser_ = authorizedUser;
        hookChainId_ = hookChainId;
        zkClient = IZkLightClient(_zkClient);
        sharedLiquidityLedger = ISharedLiquidityLedger(_sharedLiquidityLedger);
        Hooks.validateHookPermissions(this, permissions);
    }

    /*//////////////////////////////////////////////////////////////
                               MODIFIERS
    //////////////////////////////////////////////////////////////*/

    /// @notice Modifier to restrict access to authorized user
    modifier onlyAuthorizedUser() {
        require(msg.sender == authorizedUser_, "CrossSwap: Unauthorized access");
        _;
    }
}
