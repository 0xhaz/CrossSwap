// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {PoolManager} from "@uniswap/v4-core/src/PoolManager.sol";
import {DeployMerkleTree} from "script/DeployMerkleTree.s.sol";
import {DeployZKVerifier} from "script/DeployZkVerifier.s.sol";
import {DeployZkLightClient} from "script/DeployZkLightClient.s.sol";
import {DeploySharedLedger} from "script/DeploySharedLedger.s.sol";
import {Script, console2} from "forge-std/Script.sol";

/**
 * @title HelperConfig
 * @dev Helper contract for managing network configurations dynamically.
 */
contract HelperConfig is Script {
    NetworkConfig public activeNetworkConfig;

    /// @dev Struct to store configuration per network
    struct NetworkConfig {
        address poolManager;
        uint256 deployerKey;
        uint256 hookChainId;
        address zkClient;
        address sharedLedger;
    }

    /// @dev Mapping from chain ID to network configuration
    mapping(uint256 => NetworkConfig) public chainIdToNetworkConfig;

    /// @dev Default Anvil private key
    uint256 public constant DEFAULT_ANVIL_PRIVATE_KEY =
        0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;

    uint256 public chainId = block.chainid;

    uint256 sepoliaChainId = 1155111;
    uint256 bscChainId = 56;
    uint256 expChainId = 131;

    /**
     * @dev Constructor determines which network configuration to use.
     */
    constructor() {
        if (chainId == sepoliaChainId) {
            activeNetworkConfig = getSepoliaConfig();
        } else if (chainId == bscChainId) {
            activeNetworkConfig = getBSCConfig();
        } else if (chainId == expChainId) {
            activeNetworkConfig = getEXPConfig();
        } else {
            activeNetworkConfig = getOrCreateAnvilEthConfig();
        }
    }

    /**
     * @notice Returns Sepolia testnet configuration
     * @return The network configuration for Sepolia
     */
    function getSepoliaConfig() public returns (NetworkConfig memory) {
        address sharedLedgerAddress = address(new DeploySharedLedger().run());

        return NetworkConfig({
            poolManager: address(0xCa6DBBe730e31fDaACaA096821199EEED5AD84aE),
            deployerKey: vm.envUint("PRIVATE_KEY"),
            hookChainId: sepoliaChainId,
            zkClient: address(0x2dDf08e397541721acD82E5b8a1D0775454a180B),
            sharedLedger: sharedLedgerAddress
        });
    }

    /**
     * @notice Returns BSC configuration
     * @return The network configuration for BSC Mainnet
     */
    function getBSCConfig() public returns (NetworkConfig memory) {
        address sharedLedgerAddress = address(new DeploySharedLedger().run());

        return NetworkConfig({
            poolManager: address(0xCa6DBBe730e31fDaACaA096821199EEED5AD84aE),
            deployerKey: vm.envUint("PRIVATE_KEY"),
            hookChainId: bscChainId,
            zkClient: address(0x2dDf08e397541721acD82E5b8a1D0775454a180B),
            sharedLedger: sharedLedgerAddress
        });
    }

    function getEXPConfig() public returns (NetworkConfig memory) {
        address sharedLedgerAddress = address(new DeploySharedLedger().run());

        return NetworkConfig({
            poolManager: address(0xCa6DBBe730e31fDaACaA096821199EEED5AD84aE),
            deployerKey: vm.envUint("PRIVATE_KEY"),
            hookChainId: expChainId,
            zkClient: address(0x2dDf08e397541721acD82E5b8a1D0775454a180B),
            sharedLedger: sharedLedgerAddress
        });
    }

    /**
     * @notice Returns or creates Anvil configuration for local testing.
     * @return The network configuration for Anvil (local development)
     */
    function getOrCreateAnvilEthConfig() public returns (NetworkConfig memory) {
        // Check if already initialized
        if (chainIdToNetworkConfig[chainId].poolManager != address(0)) {
            return chainIdToNetworkConfig[chainId];
        }

        // Deploy new instances for Anvil
        DeploySharedLedger sharedLedgerDeployer = new DeploySharedLedger();
        DeployZkLightClient zkLightClientDeployer = new DeployZkLightClient();

        address sharedLedgerAddress = address(sharedLedgerDeployer.run());
        address zkClientAddress = address(zkLightClientDeployer.run());

        NetworkConfig memory anvilConfig = NetworkConfig({
            poolManager: address(0xCa6DBBe730e31fDaACaA096821199EEED5AD84aE), // Placeholder address
            deployerKey: DEFAULT_ANVIL_PRIVATE_KEY,
            hookChainId: 31337, // Hardhat/Anvil Chain ID
            zkClient: zkClientAddress,
            sharedLedger: sharedLedgerAddress
        });

        chainIdToNetworkConfig[chainId] = anvilConfig;
        return anvilConfig;
    }

    function getDeployedAddress() public returns (address MerkleTree, address zkLightClient, address sharedLedger) {
        DeploySharedLedger sharedLedgerDeployer = new DeploySharedLedger();
        DeployMerkleTree merkleTreeDeployer = new DeployMerkleTree();
        DeployZkLightClient zkLightClientDeployer = new DeployZkLightClient();

        address sharedLedgerAddress = address(sharedLedgerDeployer.run());
        address merkleTreeAddress = address(merkleTreeDeployer.run());
        address zkLightClientAddress = address(zkLightClientDeployer.run());

        return (merkleTreeAddress, zkLightClientAddress, sharedLedgerAddress);
    }
}
