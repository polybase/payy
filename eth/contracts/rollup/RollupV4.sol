// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "./RollupV3.sol";

contract RollupV4 is RollupV3 {
    function initializeV4() public reinitializer(4) {
        version = 4;
    }

    function containsRootHashes(
        bytes32[6] memory hashes
    ) public view override returns (bool) {
        bool[6] memory results = [false, false, false, false, false, false];

        for (uint i = 0; i < hashes.length; i++) {
            for (uint j = 0; j < rootHashes.length; j++) {
                if (hashes[i] == rootHashes[j] || hashes[i] == 0) {
                    results[i] = true;
                    break;
                }
            }
        }

        for (uint i = 0; i < results.length; i++) {
            if (results[i] == false) {
                return false;
            }
        }

        return true;
    }
}
