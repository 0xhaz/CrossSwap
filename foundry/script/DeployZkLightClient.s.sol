// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {ZkLightClient} from "src/bridge/ZkLightClient.sol";

contract DeployZkLightClient is Script {
    mapping(uint256 => address) public zkBridgeAddresses;

    constructor() {
        zkBridgeAddresses[1155111] = 0x2dDf08e397541721acD82E5b8a1D0775454a180B;
        zkBridgeAddresses[56] = 0x2dDf08e397541721acD82E5b8a1D0775454a180B;
        zkBridgeAddresses[131] = 0xa8a4547Be2eCe6Dde2Dd91b4A5adFe4A043b21C7;
    }

    function run() external returns (address) {
        vm.startBroadcast();
        uint256 chainId = block.chainid;
        require(zkBridgeAddresses[chainId] != address(0), "DeployZkLightClient: Invalid chain ID");

        address zkBridge = zkBridgeAddresses[chainId];

        ZkLightClient zkLightClient = new ZkLightClient(zkBridge);
        vm.stopBroadcast();

        console2.log("ZkLightClient deployed at address:", address(zkLightClient));

        return address(zkLightClient);
    }
}
