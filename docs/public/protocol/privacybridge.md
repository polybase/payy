# PrivacyBridge

The Privacy Bridge provides an EVM interface to the native ERC-20 privacy pools. Calls to the privacy bridge are zero rated, incentivising usage and enhancing usability for privacy flows. The privacy bridge maintains a virtual state representing the privacy layer sparse merkle tree.

The bridge verifies privacy proofs through the [Privacy Proof Verify](precompiles.md#privacy-proof-verify) precompile and reads / updates sparse merkle tree state through the [Rollup](rollup.md) predeploy.

{% hint style="info" %}
All calls to the PrivacyBridge are gas zero rated to enable [zero fee private payments](../stablecoins/zero-fee-payments.md).
{% endhint %}

If you need to construct PrivacyBridge ZK proofs manually outside the Payy SDK, see the [Manual proof construction](privacy-layer/zk-circuits.md#manual-proof-construction) section in [ZK Circuits](privacy-layer/zk-circuits.md) for the required `@aztec/bb.js` version and circuit source links.

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title PrivacyBridge Interface
/// @notice Interface for moving funds into, within, and out of the Payy native privacy pool using ZK proofs.
/// @dev Merkle tree hashing uses Poseidon. Merkle path sibling order is left-to-right at each level.
///      All proof-verification functions expect the verifier to reconstruct and validate circuit public
///      inputs from `publicInputs`.
interface IPrivacyBridge {
    // ----------------------------
    // Events
    // ----------------------------

    /// @notice Emitted when a transfer within the privacy pool is processed.
    /// @param txHash Hash of the proof inputs for off-chain correlation (implementation-defined)
    /// @param verificationKeyHash Hash of the verification key used
    event TransferProcessed(
        bytes32 indexed txHash,
        bytes32 indexed verificationKeyHash
    );

    /// @notice Emitted when funds are burned (withdrawn) from the privacy pool.
    /// @param txHash Hash of the proof inputs for off-chain correlation (implementation-defined)
    /// @param verificationKeyHash Hash of the verification key used
    event BurnProcessed(
        bytes32 indexed txHash,
        bytes32 indexed verificationKeyHash
    );

    /// @notice Emitted when funds are minted (deposited) into the privacy pool.
    /// @param txHash Hash of the proof inputs for off-chain correlation (implementation-defined)
    /// @param verificationKeyHash Hash of the verification key used
    event MintProcessed(
        bytes32 indexed txHash,
        bytes32 indexed verificationKeyHash
    );

    /// @notice Emitted when ERC-20 funds are swept into bridge custody.
    /// @param sweepHash Hash derived from keccak256(abi.encodePacked(to, nonce)) authorizing the sweep
    /// @param token ERC-20 token address that was swept
    /// @param from The visible ERC-20 account that funds were moved from (equals msg.sender)
    /// @param amount The actual amount swept (<= maxAmount)
    event SweepProcessed(
        bytes32 indexed sweepHash,
        address indexed token,
        address indexed from,
        uint256 amount
    );

    // ----------------------------
    // Core functions
    // ----------------------------

    /// @notice Move funds within the privacy pool.
    /// @dev Expected public inputs (canonical ordering, implementation-defined):
    ///      - recent_root
    ///      - input_nullifiers_x2
    ///      - output_commitments_x2
    ///      - `recent_root` must be contained in Rollup's recent-root window
    function transfer(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs
    ) external;

    /// @notice Move funds out of the privacy pool (withdraw/burn).
    /// @dev Expected public inputs (canonical ordering, implementation-defined):
    ///      - recent_root
    ///      - input_nullifiers_x2
    ///      - output_commitments_x2
    ///      - burn_recipient_public
    ///      - burn_value
    ///      - `recent_root` must be contained in Rollup's recent-root window
    function burn(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs
    ) external;

    /// @notice Move funds into the privacy pool (deposit/mint).
    /// @dev Expected public inputs (canonical ordering, implementation-defined):
    ///      - recent_root
    ///      - input_nullifiers_x2
    ///      - output_commitments_x2
    ///      - mint_value
    ///      - `recent_root` must be contained in Rollup's recent-root window
    function mint(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs
    ) external;

    /// @notice Sweep ERC-20 funds from msg.sender into bridge custody up to a maximum amount.
    /// @dev
    /// - Source: the visible source account is msg.sender.
    /// - Authorization: `sweepHash` uniquely identifies the sweep and prevents replay.
    /// - Amount: implementation will transfer `min(balanceOf(msg.sender), allowance(msg.sender, this), maxAmount)` of
    ///   `token` into bridge custody.
    /// - Accounting: implementation emits `SweepProcessed` with the actual amount swept.
    /// - Note: the current implementation does not mint a private commitment as part of `sweep`.
    /// @param token The ERC-20 token to sweep
    /// @param sweepHash Hash derived from keccak256(abi.encodePacked(to, nonce))
    /// @param maxAmount Maximum amount to sweep
    function sweep(
        address token,
        bytes32 sweepHash,
        uint256 maxAmount
    ) external;

    // ----------------------------
    // Introspection / helpers
    // ----------------------------

    /// @notice Returns true if a commitment / nullifier has already been inserted into the rollup tree.
    /// @dev Convenience helper over `Rollup.exists(bytes32)`.
    function elementExists(bytes32 element) external view returns (bool);

    /// @notice Optional helper to compute a canonical hash of proof inputs for event indexing.
    /// @dev Implementations may define txHash as keccak256(abi.encode(verificationKeyHash, proof, publicInputs)).
    function computeTxHash(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs
    ) external pure returns (bytes32);

    /// @notice Returns the Merkle inclusion path for a given commitment.
    /// @dev Tree uses Poseidon hashing. Sibling order is strictly left-to-right per level.
    function getMerklePath(
        bytes32 commitment
    ) external view returns (
        bytes32 root,
        bytes32[] memory siblings
    );

    /// @notice Returns the current Merkle root recognized by the bridge.
    function getRoot() external view returns (bytes32 root);
}

```

`elementExists(bytes32)` is part of the deployed `PrivacyBridge` implementation and forwards to `Rollup.exists(bytes32)` for O(1) existence checks before insertions.
