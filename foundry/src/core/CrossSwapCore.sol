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
// import {CCLib} from "src/libraries/CCLib.sol";
import {console2} from "forge-std/Test.sol";

abstract contract CrossSwapCore is BaseHook, IZkLightClient, ISharedLiquidityLedger {
    using CurrencyLibrary for Currency;
    using CurrencySettler for Currency;
    using PoolIdLibrary for PoolKey;
    using SafeCast for uint256;
    using SafeCast for uint128;
    using StateLibrary for IPoolManager;

    /*//////////////////////////////////////////////////////////////
                              STORAGE VARIABLES
    //////////////////////////////////////////////////////////////*/
    // Mappping of hook's chain ID
    uint256 public hookChainId_;

    // Authorized user
    address public authorizedUser_;

    uint256 public constant TREE_DEPTH = 32;

    ISharedLiquidityLedger public sharedLiquidityLedger;
    IZkLightClient public zkClient;

    /*//////////////////////////////////////////////////////////////
                               MODIFIERS
    //////////////////////////////////////////////////////////////*/

    /// @notice Modifier for functions that can only be called by the poolManager
    modifier poolManagerOnly() {
        if (msg.sender != address(poolManager)) revert Errors.NotPoolManager();
        _;
    }

    /// @notice Modifier to restrict access to authorized user
    modifier onlyAuthorizedUser() {
        require(msg.sender == authorizedUser_, "CrossSwap: Unauthorized access");
        _;
    }

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
    ) BaseHook(poolManager) {
        authorizedUser = authorizedUser_;
        zkClient = IZkLightClient(zkClient_);
        sharedLiquidityLedger = ISharedLiquidityLedger(sharedLiquidityLedger_);
        hookChainId_ = hookChainId;
    }

    /*//////////////////////////////////////////////////////////////
                                 HOOKS
    //////////////////////////////////////////////////////////////*/

    function getHookPermissions() public pure override returns (Hooks.Permissions memory) {
        return Hooks.Permissions({
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
        });
    }

    /*//////////////////////////////////////////////////////////////
                             HOOK FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    /// @inheritdoc IHooks
    function beforeAddLiquidity(address, PoolKey calldata, IPoolManager.ModifyLiquidityParams calldata, bytes calldata)
        external
        virtual
        override
        returns (bytes4)
    {
        revert Errors.HookNotImplemented();
    }

    /// @inheritdoc IHooks
    function afterAddLiquidity(
        address,
        PoolKey calldata,
        IPoolManager.ModifyLiquidityParams calldata,
        BalanceDelta,
        BalanceDelta,
        bytes calldata
    ) external virtual override returns (bytes4, BalanceDelta) {
        revert Errors.HookNotImplemented();
    }

    /// @inheritdoc IHooks
    function beforeRemoveLiquidity(
        address,
        PoolKey calldata,
        IPoolManager.ModifyLiquidityParams calldata,
        bytes calldata
    ) external virtual override returns (bytes4) {
        revert Errors.HookNotImplemented();
    }

    /// @inheritdoc IHooks
    function afterRemoveLiquidity(
        address,
        PoolKey calldata,
        IPoolManager.ModifyLiquidityParams calldata,
        BalanceDelta,
        BalanceDelta,
        bytes calldata
    ) external virtual override returns (bytes4, BalanceDelta) {
        revert Errors.HookNotImplemented();
    }

    /// @inheritdoc IHooks
    function beforeSwap(address, PoolKey calldata, IPoolManager.SwapParams calldata, bytes calldata)
        external
        virtual
        override
        returns (bytes4, BeforeSwapDelta, uint24)
    {
        revert Errors.HookNotImplemented();
    }

    /// @inheritdoc IHooks
    function afterSwap(address, PoolKey calldata, IPoolManager.SwapParams calldata, BalanceDelta, bytes calldata)
        external
        virtual
        override
        returns (bytes4, int128)
    {
        revert Errors.HookNotImplemented();
    }

    /*//////////////////////////////////////////////////////////////
                        OVERRIDE FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function updateLiquidityState(uint256 chainId, bytes32 newStateRoot, bytes memory zkProof)
        public
        override
        onlyAuthorizedUser
    {
        sharedLiquidityLedger.updateLiquidityState(chainId, newStateRoot, zkProof);
    }

    function getLatestLiquidityState(uint256 chainId) public view override returns (bytes32) {
        return sharedLiquidityLedger.getLatestLiquidityState(chainId);
    }

    function getLatestLiquidityProof(uint256 chainId) public view override returns (bytes memory) {
        return sharedLiquidityLedger.getLatestLiquidityProof(chainId);
    }

    function insert(bytes32 leaf) public override onlyAuthorizedUser returns (bytes32 newRoot) {
        return sharedLiquidityLedger.insert(leaf);
    }

    function getMerkleProof(uint256 index) public view override returns (bytes32[TREE_DEPTH] memory proof) {
        return sharedLiquidityLedger.getMerkleProof(index);
    }

    function verifyProof(bytes32 leaf, bytes32[TREE_DEPTH] memory proof, bytes32 root, uint256 index)
        public
        view
        override
        returns (bool)
    {
        return sharedLiquidityLedger.verifyProof(leaf, proof, root, index);
    }

    function getMerkleRoot() public view override returns (bytes32) {
        return sharedLiquidityLedger.getMerkleRoot();
    }

    function sendMessage(uint16 dstChainId, address dstHook, bytes memory payload)
        public
        payable
        override
        returns (uint64)
    {
        return zkClient.sendMessage(dstChainId, dstHook, payload);
    }

    function zkReceive(uint256 srchChainId, address srcAddress, uint64 nonce, bytes memory payload) external override {
        zkClient.zkReceive(srchChainId, srcAddress, nonce, payload);
    }

    function estimateFee(uint16 dstChainId) public view override returns (uint256) {
        return zkClient.estimateFee(dstChainId);
    }

    function stateTree() external view override returns (IMerkleTree) {
        return sharedLiquidityLedger.stateTree();
    }

    function currentIndex() public view override returns (uint256) {
        return sharedLiquidityLedger.currentIndex();
    }

    function verifyLiquidityProof(
        uint256[2] memory proofA,
        uint256[2][2] memory proofB,
        uint256[2] memory proofC,
        uint256[4] memory publicSignals
    ) external view override returns (bool) {
        return sharedLiquidityLedger.verifyLiquidityProof(proofA, proofB, proofC, publicSignals);
    }

    function verifySwapProof(
        uint256[2] memory proofA,
        uint256[2][2] memory proofB,
        uint256[2] memory proofC,
        uint256[5] memory publicSignals
    ) external view override returns (bool) {
        return sharedLiquidityLedger.verifySwapProof(proofA, proofB, proofC, publicSignals);
    }
}
