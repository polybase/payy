// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import "./RollupV1.sol";

contract RollupV2 is RollupV1 {
    function initializeV2() public reinitializer(2) {
        version = 2;
    }
}
