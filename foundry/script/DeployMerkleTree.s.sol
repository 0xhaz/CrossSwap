// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {MerkleTree} from "src/zk/MerkleTree.sol";

contract DeployMerkleTree is Script {
    function run() external returns (address) {
        vm.startBroadcast();
        MerkleTree tree = new MerkleTree();
        vm.stopBroadcast();

        console2.log("Merkle Tree deployed at address:", address(tree));

        return address(tree);
    }
}
