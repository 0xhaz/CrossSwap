// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

import {Test, console2} from "forge-std/Test.sol";
import {SharedLiquidityLedger} from "src/zk/SharedLiquidityLedger.sol";
import {DeploySharedLedger} from "script/DeploySharedLedger.s.sol";
import {MerkleTree} from "src/zk/MerkleTree.sol";
import {ZKVerifier} from "src/zk/ZKVerifier.sol";

contract SharedLiquidityTest is Test {
    SharedLiquidityLedger ledger;
    MerkleTree stateTree;
    ZKVerifier zkVerifier;

    function setUp() public {
        DeploySharedLedger deployer = new DeploySharedLedger();
        address ledgerAddr = deployer.run(); // No broadcast, avoids error in Foundry tests
        ledger = SharedLiquidityLedger(ledgerAddr);

        // zkVerifier = ledger.zkVerifier();
        // stateTree = ledger.stateTree();
    }
}
