// SPDX-License-Identifier: SEE LICENSE IN LICENSE
pragma solidity ^0.8.26;

contract MockZkLightClient {
    event MessageReceived(uint16 srcChainId, bytes srcAddress, uint64 nonce, bytes payload);

    function send(uint16 srcChainId, bytes memory srcAddress, uint64 nonce, bytes memory payload) external {
        emit MessageReceived(srcChainId, srcAddress, nonce, payload);
    }

    function verifyProof(bytes memory proof) external pure returns (bool) {
        return keccak256(proof) != keccak256(bytes("INVALID"));
    }
}
