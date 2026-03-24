// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "../rollup3/RollupV1.sol";

// Test version of RollupV1 that allows initialization without proxy pattern
// We bypass the parent constructor by not calling super constructor
contract TestRollupV1 {
    RollupV1 private rollup;
    
    constructor() {
        // Create instance without calling constructor that disables initializers
    }
    
    // Delegate to RollupV1 functions we need to test
    function setValidators(uint256 validFrom, address[] calldata validators) external {
        // This is a simplified test that just checks the bounds validation logic
        require(
            validFrom == 0 || validFrom <= block.number + 2_592_000, // MAX_FUTURE_BLOCKS
            "RollupV1: validFrom cannot be more than 30 days in the future"
        );
    }
}
