// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Script, console2} from "forge-std/Script.sol";
import {DeployMerkleTree} from "script/DeployMerkleTree.s.sol";
import {DeployZKVerifier} from "script/DeployZkVerifier.s.sol";
import {SharedLiquidityLedger} from "src/zk/SharedLiquidityLedger.sol";

contract DeploySharedLedger is Script {
    function run() external returns (address) {
        DeployZKVerifier deployZkVerifier = new DeployZKVerifier();
        address zkVerifier = deployZkVerifier.run();
        vm.startBroadcast();

        SharedLiquidityLedger deployLedger = new SharedLiquidityLedger(zkVerifier);
        address ledger = address(deployLedger);

        console2.log("Shared Ledger deployed at address:", address(ledger));
        vm.stopBroadcast();

        return address(ledger);
    }
}
