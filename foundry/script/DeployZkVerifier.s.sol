// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {ZKVerifier} from "src/zk/ZKVerifier.sol";
import {DeployMerkleTree} from "script/DeployMerkleTree.s.sol";
import {IMerkleTree} from "src/interfaces/IMerkleTree.sol";

contract DeployZKVerifier is Script {
    function run() external returns (address) {
        vm.startBroadcast();

        ZKVerifier verifier = new ZKVerifier();
        console2.log("ZK Verifier deployed at address:", address(verifier));

        vm.stopBroadcast();

        return address(verifier);
    }
}
