# PrivacyVaultRegistry

The PrivacyVaultRegistry is used in the transparently upgraded private transfer flow to determine which privacy vault service a user is using:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title IPrivacyVaultRegistry
/// @notice Interface for a registry that maps an owner to a canonical Privacy Vault URI.
interface IPrivacyVaultRegistry {
    // ----------------------------
    // Events
    // ----------------------------

    /// @notice Emitted when a vault URI is created for an owner.
    /// @param owner The owner of the vault record
    /// @param uri The URI that was set
    event VaultCreated(address indexed owner, string uri);

    /// @notice Emitted when a vault URI is updated.
    /// @param owner The owner of the vault record
    /// @param oldUri The previous URI
    /// @param newUri The new URI that was set
    event VaultUpdated(address indexed owner, string oldUri, string newUri);

    /// @notice Emitted when ownership of a vault record is transferred.
    /// @param previousOwner The previous owner
    /// @param newOwner The new owner
    event VaultOwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    /// @notice Emitted when a vault record is removed.
    /// @param owner The owner whose record was removed
    /// @param oldUri The URI that was removed
    event VaultRemoved(address indexed owner, string oldUri);

    // ----------------------------
    // Core functions
    // ----------------------------

    /// @notice Create a new vault record for msg.sender with the provided URI.
    /// @param uri The URI string to associate with the caller's vault
    function createVault(string calldata uri) external;

    /// @notice Update the caller-owned vault URI.
    /// @param newUri The new URI string
    function updateVault(string calldata newUri) external;

    /// @notice Remove the caller-owned vault record.
    function removeVault() external;

    /// @notice Transfer ownership of the caller-owned vault record to a new owner.
    /// @param newOwner The address of the new owner
    function transferVaultOwnership(address newOwner) external;

    // ----------------------------
    // Views
    // ----------------------------

    /// @notice Get the vault URI for a given owner.
    /// @param owner The owner address to query
    /// @return uri The URI string associated with the owner (empty if none)
    function getVaultURI(address owner) external view returns (string memory uri);

    /// @notice Returns true if a record exists for the given owner.
    /// @param owner The owner address to query
    /// @return exists Whether the record exists
    function hasVault(address owner) external view returns (bool exists);

    /// @notice Returns metadata for a vault record.
    /// @param owner The owner address to query
    /// @return uri The URI string
    /// @return recordOwner The current owner/controller of the record
    /// @return createdAt Creation timestamp
    /// @return updatedAt Last update timestamp
    /// @return exists Whether the record exists
    function getVaultRecord(address owner)
        external
        view
        returns (
            string memory uri,
            address recordOwner,
            uint64 createdAt,
            uint64 updatedAt,
            bool exists
        );
}

```
