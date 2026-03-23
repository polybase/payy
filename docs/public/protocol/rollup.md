# Rollup

The Rollup contract is a genesis deployed wrapper around the Privacy Layer sparse merkle tree.
The contract maintains a FIFO ring-buffer of the last 1024 roots to support recent-root checks.

The protocol provides this for other developers to use in their own privacy rollups.

The low-level Smirk precompiles consumed by this wrapper are documented in [Precompiles](precompiles.md).

The following shows the Solidity interface for the Rollup contract:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IRollup {
    // ----------------------------
    // Core functions
    // ----------------------------

    /// @notice Add an element to the rollup tree.
    function add(bytes32 data) external;

    /// @notice Remove an element from the rollup tree.
    function remove(bytes32 data) external;

    /// @notice Check whether an element exists.
    function exists(bytes32 data) external view returns (bool);

    /// @notice Get the merkle path siblings for an already derived tree element.
    function getMerklePath(bytes32 element) external view returns (bytes32[] memory);

    /// @notice Get the current rollup root.
    function getRoot() external view returns (bytes32);

    /// @notice Returns true if `root` is in the recent-root window.
    /// @dev The current root is always treated as recent.
    function isRecentRoot(bytes32 root) external view returns (bool);

    /// @notice The fixed FIFO capacity for recent roots.
    function recentRootsCapacity() external pure returns (uint256);

    /// @notice Current number of tracked recent roots (saturates at 1024).
    function recentRootsCount() external view returns (uint256);
}
```
