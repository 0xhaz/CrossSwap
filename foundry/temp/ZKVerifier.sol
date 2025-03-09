// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.24;

import {IZKVerifier} from "./IZKVerifier.sol";

/// @title ZK Verifier for Private Liquidity & Cross-Chain Swaps
/// @notice This contract verifies zero-knowledge proofs for liquidity and swap operations
contract ZKVerifier is IZKVerifier {
    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;
    uint16 constant pLastMem = 896;

    // Scalar field size
    uint256 constant r = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    // Base field size
    uint256 constant q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;

    // Verification Key data (Swap)
    uint256 constant swapAlphax = 20491192805390485299153009773594534940189261866228447918068658471970481763042;
    uint256 constant swapAlphay = 9383485363053290200918347156157836566562967994039712273449902621266178545958;

    uint256 constant swapBetax1 = 6375614351688725206403948262868962793625744043794305715222011528459656738731;
    uint256 constant swapBetax2 = 4252822878758300859123897981450591353533073413197771768651442665752259397132;
    uint256 constant swapBetay1 = 10505242626370262277552901082094356697409835680220590971873171140371331206856;
    uint256 constant swapBetay2 = 21847035105528745403288232691147584728191162732299865338377159692350059136679;

    uint256 constant swapGammax1 = 10857046999023057135944570762232829481370756359578518086990519993285655852781;
    uint256 constant swapGammax2 = 11559732032986387107991004021392285783925812861821192530917403151452391805634;
    uint256 constant swapGammay1 = 8495653923123431417604973247489272438418190587263600148770280649306958101930;
    uint256 constant swapGammay2 = 4082367875863433681332203403145435568316851327593401208105741076214120093531;

    uint256 constant swapDeltax1 = 17105467859765154924693659042454546093694324380130295194078386656526418666290;
    uint256 constant swapDeltax2 = 2191097159270147648338883173190117281428039837237329990521350763513483143246;
    uint256 constant swapDeltay1 = 11568753648221023598310564321345473556704234310287731501445379034809807722750;
    uint256 constant swapDeltay2 = 11128938518318450754282049425021251623933388487560942216280434896585010455789;

    uint256 constant IC0x = 2469647548574162777686952854781462740570291371705225168395084449385407326151;
    uint256 constant IC0y = 13193723657107369465180604521279526129998093986967529880838930142786731121547;

    uint256 constant IC1x = 12684546782647836100294740987977388529213971554526680541684154242527490724551;
    uint256 constant IC1y = 73586102199756904508275186467033970703275246518953228044031542835272986303;

    uint256 constant IC2x = 18589664365221704487850596508150525528833305438151330764923769942892462257624;
    uint256 constant IC2y = 4980618336075941461102752903864278770000931291175213480987831242393011334961;

    uint256 constant IC3x = 12376396872858105565570240026004554912151757160175073704120365378134344340978;
    uint256 constant IC3y = 6138293021242234002706513605148623881168699131247527975854574962758048681910;

    uint256 constant IC4x = 2167644399407268036705537211385699076991092888147116831798066334825561022557;
    uint256 constant IC4y = 18988150335532852560789518568608002627612000661901883027344951443749505077348;

    uint256 constant IC5x = 12979962827968287101122612929981782689712200640466949222584699875226259198859;
    uint256 constant IC5y = 821924551716058989006731614321128815001334815047579119952025162436232170951;

    // Verification Key data (Liquidity)
    uint256 constant liquidityAlphax = 20491192805390485299153009773594534940189261866228447918068658471970481763042;
    uint256 constant liquidityAlphay = 9383485363053290200918347156157836566562967994039712273449902621266178545958;
    uint256 constant liquidityDeltax1 = 16092470943075564864006572325597660854947911315146660370786593756034615435123;
    uint256 constant liquidityDeltax2 = 6683220847743756137660483811837905269459861644489682969654951914545139661979;
    uint256 constant liquidityDeltay1 = 3729574437916110239719984869913959442070234274092880449770594090589377509291;
    uint256 constant liquidityDeltay2 = 6102004072327578896573919153118876654159291851381382716384312758843418221287;

    event DebugVkX(uint256 x, uint256 y);
    event DebugPairing(bool success, uint256 result);

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
    ) external returns (bool) {
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
    ) internal returns (bool) {
        assembly {
            function modReduce(v) -> rV {
                rV := mod(v, r)
            }

            function checkField(v) {
                if iszero(lt(v, r)) {
                    mstore(0, 0)
                    return(0, 0x20)
                }
            }

            function g1_mulAccC(pR, x, y, s) {
                let success
                let mIn := mload(0x40)
                mstore(mIn, x)
                mstore(add(mIn, 32), y)
                mstore(add(mIn, 64), s)
                success := staticcall(sub(gas(), 2000), 7, mIn, 96, mIn, 64)
                if iszero(success) { revert(0, 0) }
                mstore(add(mIn, 64), mload(pR))
                mstore(add(mIn, 96), mload(add(pR, 32)))
                success := staticcall(sub(gas(), 2000), 6, mIn, 128, pR, 64)
                if iszero(success) { revert(0, 0) }
            }

            function checkPairing(pA, pB, pC, pubSignals, pMem) -> isOk {
                let _pPairing := add(pMem, pPairing)
                let _pVk := add(pMem, pVk)

                // Compute vk_x (for consistency)
                mstore(_pVk, IC0x)
                mstore(add(_pVk, 32), IC0y)
                g1_mulAccC(_pVk, IC1x, IC1y, modReduce(calldataload(add(pubSignals, 0))))
                g1_mulAccC(_pVk, IC2x, IC2y, modReduce(calldataload(add(pubSignals, 32))))
                g1_mulAccC(_pVk, IC3x, IC3y, modReduce(calldataload(add(pubSignals, 64))))
                g1_mulAccC(_pVk, IC4x, IC4y, modReduce(calldataload(add(pubSignals, 96))))
                g1_mulAccC(_pVk, IC5x, IC5y, modReduce(calldataload(add(pubSignals, 128))))

                // Debug vk_x
                mstore(0x00, mload(_pVk))
                mstore(0x20, mload(add(_pVk, 32)))
                log1(0x00, 0x40, 0xcd5376336434485e3bfba6c77cf5b86c05703f876d14a49beef82912866f475d)

                // Pair G1 generator [1, 2] with Beta
                mstore(_pPairing, 1)
                mstore(add(_pPairing, 32), 2)
                mstore(add(_pPairing, 64), swapBetax1)
                mstore(add(_pPairing, 96), swapBetax2)
                mstore(add(_pPairing, 128), swapBetay1)
                mstore(add(_pPairing, 160), swapBetay2)

                let success := staticcall(sub(gas(), 2000), 8, _pPairing, 192, _pPairing, 0x20)
                mstore(0x00, success)
                mstore(0x20, mload(_pPairing))
                log1(0x00, 0x40, 0xd4f1e83a3324f0217524cd800dce548e9aad2383912c50ea1646ac7534bd7a96)

                isOk := and(success, mload(_pPairing))
            }

            let pMem := mload(0x40)
            mstore(0x40, add(pMem, pLastMem))

            checkField(calldataload(add(_pubSignals, 0)))
            checkField(calldataload(add(_pubSignals, 32)))
            checkField(calldataload(add(_pubSignals, 64)))
            checkField(calldataload(add(_pubSignals, 96)))
            checkField(calldataload(add(_pubSignals, 128)))

            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals, pMem)
            mstore(0, isValid)
            return(0, 0x20)
        }
    }
}
