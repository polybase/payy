// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "../noir/IVerifier.sol";

contract MockVerifier is IVerifier {
    function verify(
        bytes calldata proof,
        bytes32[] calldata publicInputs
    ) external pure override returns (bool) {
        // Always return true for testing purposes
        return true;
    }
}