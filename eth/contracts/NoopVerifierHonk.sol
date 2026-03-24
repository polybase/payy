// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

interface IVerifier {
    function verify(
        bytes calldata _proof,
        bytes32[] calldata _publicInputs
    ) external view returns (bool);
}

contract HonkVerifier is IVerifier {
    function verify(
        bytes calldata /* _proof */,
        bytes32[] calldata /* _publicInputs */
    ) external pure override returns (bool) {
        return true;
    }
}
