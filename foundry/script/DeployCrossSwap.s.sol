// // SPDX-License-Identifier: SEE LICENSE IN LICENSE
// pragma solidity ^0.8.26;

// import {Script, console2} from "forge-std/Script.sol";
// import {DeploySharedLedger} from "script/DeploySharedLedger.s.sol";
// import {CrossSwap} from "src/CrossSwap.sol";
// import {IPoolManager} from "@uniswap/v4-core/src/interfaces/IPoolManager.sol";
// import {HelperConfig} from "script/HelperConfig.s.sol";
// import {ZkLightClient} from "src/bridge/ZkLightClient.sol";

// contract DeployCrossSwap is Script {
//     function run() public returns (address) {
//         HelperConfig helperConfig = new HelperConfig();

//         (address poolManager, uint256 deployerKey, uint256 hookChainId, address zkClient, address sharedLedger) =
//             helperConfig.activeNetworkConfig();

//         vm.startBroadcast();

//         CrossSwap crossSwap = new CrossSwap(
//             IPoolManager(poolManager),
//             address(uint160(deployerKey)),
//             hookChainId,
//             ZkLightClient(payable(zkClient)),
//             sharedLedger
//         );

//         vm.stopBroadcast();

//         console2.log(" CrossSwap deployed at:", address(crossSwap));

//         return address(crossSwap);
//     }
// }
