// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.24;

import {IZKVerifier} from "src/interfaces/IZKVerifier.sol";

/// @title ZK Verifier for Private Liquidity & Cross-Chain Swaps
/// @notice This contract verifies zero-knowledge proofs for liquidity and swap operations
contract ZKVerifier is IZKVerifier {
    // Scalar field size
    uint256 constant r = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    // Base field size
    uint256 constant q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;

    // Verification Key data (Swap)
    uint256 constant swapAlphax = 20491192805390485299153009773594534940189261866228447918068658471970481763042;
    uint256 constant swapAlphay = 9383485363053290200918347156157836566562967994039712273449902621266178545958;
    uint256 constant swapDeltax1 = 2191097159270147648338883173190117281428039837237329990521350763513483143246;
    uint256 constant swapDeltax2 = 17105467859765154924693659042454546093694324380130295194078386656526418666290;
    uint256 constant swapDeltay1 = 11128938518318450754282049425021251623933388487560942216280434896585010455789;
    uint256 constant swapDeltay2 = 11568753648221023598310564321345473556704234310287731501445379034809807722750;

    // Verification Key data (Liquidity)
    uint256 constant liquidityAlphax = 20491192805390485299153009773594534940189261866228447918068658471970481763042;
    uint256 constant liquidityAlphay = 9383485363053290200918347156157836566562967994039712273449902621266178545958;
    uint256 constant liquidityDeltax1 = 16092470943075564864006572325597660854947911315146660370786593756034615435123;
    uint256 constant liquidityDeltax2 = 6683220847743756137660483811837905269459861644489682969654951914545139661979;
    uint256 constant liquidityDeltay1 = 3729574437916110239719984869913959442070234274092880449770594090589377509291;
    uint256 constant liquidityDeltay2 = 6102004072327578896573919153118876654159291851381382716384312758843418221287;

    /// @notice Verifies a zk-SNARK proof for liquidity verification (4 public signals)
    function verifyLiquidityProof(
        uint256[2] calldata proofA,
        uint256[2][2] calldata proofB,
        uint256[2] calldata proofC,
        uint256[4] calldata publicSignals
    ) external view returns (bool) {
        return _verifyGroth16Liquidity(proofA, proofB, proofC, publicSignals);
    }

    /// @notice Verifies a zk-SNARK proof for swap verification (5 public signals)
    function verifySwapProof(
        uint256[2] calldata proofA,
        uint256[2][2] calldata proofB,
        uint256[2] calldata proofC,
        uint256[5] calldata publicSignals
    ) external view returns (bool) {
        return _verifyGroth16Swap(proofA, proofB, proofC, publicSignals);
    }

    /// @notice Internal verifier function for liquidity (4 public signals)
    function _verifyGroth16Liquidity(
        uint256[2] calldata _pA,
        uint256[2][2] calldata _pB,
        uint256[2] calldata _pC,
        uint256[4] calldata _pubSignals
    ) internal view returns (bool) {
        assembly {
            function checkPairing(pA, pB, pC, pubSignals) -> isOk {
                let success := staticcall(sub(gas(), 2000), 8, pA, 768, pA, 0x20)
                isOk := and(success, mload(pA))
            }

            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals)
            mstore(0, isValid)
            return(0, 0x20)
        }
    }

    /// @notice Internal verifier function for swaps (5 public signals)
    function _verifyGroth16Swap(
        uint256[2] calldata _pA,
        uint256[2][2] calldata _pB,
        uint256[2] calldata _pC,
        uint256[5] calldata _pubSignals
    ) internal view returns (bool) {
        assembly {
            function checkPairing(pA, pB, pC, pubSignals) -> isOk {
                let success := staticcall(sub(gas(), 2000), 8, pA, 768, pA, 0x20)
                isOk := and(success, mload(pA))
            }

            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals)
            mstore(0, isValid)
            return(0, 0x20)
        }
    }
}
