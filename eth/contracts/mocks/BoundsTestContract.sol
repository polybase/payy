// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

// Simple contract to test just the bounds checking logic
contract BoundsTestContract {
    uint256 constant MAX_FUTURE_BLOCKS = 2_592_000; // 30 days (~1 sec blocks)

    // Test function that only checks the bounds logic
    function validateBounds(uint256 validFrom) external view {
        require(
            validFrom == 0 || validFrom <= block.number + MAX_FUTURE_BLOCKS,
            "BoundsTest: validFrom cannot be more than 30 days in the future"
        );
    }

    // Helper function to get current block + max future blocks
    function getMaxAllowedValidFrom() external view returns (uint256) {
        return block.number + MAX_FUTURE_BLOCKS;
    }

    // Helper function to get current block number
    function getCurrentBlock() external view returns (uint256) {
        return block.number;
    }

    // Helper function to get MAX_FUTURE_BLOCKS constant
    function getMaxFutureBlocks() external pure returns (uint256) {
        return MAX_FUTURE_BLOCKS;
    }
}