// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Verifier {
    function requireValidFieldElement(bytes32 x) internal pure {
        require(
            x <
                0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001,
            "Invalid field element"
        );
    }
}
