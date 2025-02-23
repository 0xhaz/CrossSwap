// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {ZKVerifier} from "src/zk/ZKVerifier.sol";
// import {SwapVerifier} from "src/zk/SwapVerifier.sol";

contract DeployZKVerifier is Script {
    function run() external returns (address) {
        vm.startBroadcast();
        ZKVerifier verifier = new ZKVerifier();
        // SwapVerifier verifier = new SwapVerifier();
        vm.stopBroadcast();

        console2.log("ZK Verifier deployed at address:", address(verifier));

        return address(verifier);
    }
}
