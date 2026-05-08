# PrivacyBridge

The Privacy Bridge provides the EVM interface to Payy's native ERC-20 privacy pools. Calls to the bridge are zero rated, which keeps privacy flows usable for everyday transfers. The bridge maintains a virtual view of the privacy-layer sparse Merkle tree.

The bridge verifies privacy proofs through the [Privacy Proof Verify](precompiles.md#privacy-proof-verify) precompile and reads / updates sparse Merkle tree state through the [Rollup](rollup.md) predeploy.

{% hint style="info" %}
All calls to `PrivacyBridge` are gas zero rated to enable [zero fee private payments](../stablecoins/zero-fee-payments.md).
{% endhint %}

Use the [Payy client SDKs](../build-on-payy/payy-client/README.md) for wallet-facing PrivacyBridge flows in TypeScript or Rust. If you need to construct PrivacyBridge ZK proofs manually outside the Payy SDKs, see [ZK Circuits](privacy-layer/zk-circuits.md).

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IPrivacyBridge {
    event TransferProcessed(
        bytes32 indexed txHash,
        bytes32 indexed verificationKeyHash
    );
    event BurnProcessed(
        bytes32 indexed txHash,
        bytes32 indexed verificationKeyHash
    );
    event MintProcessed(
        bytes32 indexed txHash,
        bytes32 indexed verificationKeyHash
    );
    event ExternalTransfer(bytes6 indexed prefix6, bytes32 indexed txHash);
    event ChainPublicKeyUpdated(uint256 newX, uint256 newY);

    struct TxnData {
        bytes32 verificationKeyHash;
        bytes32[5] senderEncryptedNote;
        bytes32[5] recipientEncryptedNote;
        bytes32[3] senderChainEncryptedKey;
        bytes32[3] recipientChainEncryptedKey;
        bytes32[4] userEncryptedKey;
        bytes32[4] recipientEncryptedKey;
        bytes32 memo;
    }

    function transfer(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs,
        bytes32[4] calldata userEncryptedKey,
        bytes32[4] calldata recipientEncryptedKey,
        bytes32 memo
    ) external;

    function burn(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs,
        bytes32[4] calldata userEncryptedKey
    ) external;

    function mint(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs,
        bytes32[4] calldata userEncryptedKey
    ) external;

    function updateChainPublicKey(uint256 newX, uint256 newY) external;
    function elementExists(bytes32 element) external view returns (bool);
    function computeTxHash(
        bytes32 verificationKeyHash,
        bytes calldata proof,
        bytes32[] calldata publicInputs
    ) external pure returns (bytes32);
    function getMerklePath(
        bytes32 commitment
    ) external view returns (bytes32 root, bytes32[] memory siblings);
    function getRoot() external view returns (bytes32 root);
    function getTxnHashByNonceHash(bytes32 nonceHash) external view returns (bytes32);
    function getTxnHashByCommitment(bytes32 commitment) external view returns (bytes32);
    function getTxnData(bytes32 txnHash) external view returns (TxnData memory);
    function getChainPublicKey() external view returns (uint256 x, uint256 y);
}
```

`transfer(...)` accepts both `transfer_send` and `transfer_claim` proofs.

- `transfer_send` creates a sender continuation note plus a recipient-owned incoming note
- `transfer_claim` merges an incoming note into the recipient's standard wallet chain

The proof verifier returns a distinct kind for each of those two transfer variants, and the bridge
uses that decoded kind for transfer-side branching and calldata validation.

All privacy circuits now share the same **33-field** public input layout. The bridge validates that canonical vector directly against chain state, bridge config, and calldata.

## Encryption Model

Every privacy entrypoint requires `userEncryptedKey`, which binds sender-visible decryption data to the proof.

`transfer_send` also carries:

- `recipientEncryptedKey`
- a separate recipient-encrypted note payload
- a separate chain-encrypted key for the recipient payload
- an optional fixed-width `bytes32 memo`

`transfer_send` must move non-zero value into the recipient-owned incoming note. Zero-value direct sends are rejected at the proof layer.

This means the sender continuation note and the recipient incoming note are stored and decrypted as separate domains.

`memo` is unauthenticated sender metadata. The ZK proof does not authenticate its contents, and the bridge does not treat it as trusted routing or authorization data. Integrators should only use it for optional UX or application metadata, not for any security-critical decision.

## Recipient Discovery

The standard receive path is on-chain.

For `transfer_send`, the bridge:

- stores recipient-side encrypted note material in `TxnData`
- emits `ExternalTransfer(prefix6, txHash)`

`prefix6` is derived from the first 6 bytes of the recipient owner hash. Recipient wallets filter logs by that prefix, fetch candidate `txHash` records, attempt decryption, and then claim matching notes.

No off-chain inbox publish step is required for the standard protocol flow.

## Lookup Surface

Successful `mint`, `burn`, and `transfer` calls persist:

- `nonce_hash -> txn_hash`
- `commitment -> txn_hash`
- `txn_hash -> TxnData`

That lookup surface lets wallets recover note-chain state and encrypted payloads without relying only on events. `elementExists(bytes32)` remains a convenience helper over `Rollup.exists(bytes32)`.

## Deposit Safety

Incoming token pulls for `mint(...)` must increase the bridge's token balance by exactly the requested amount. Tokens with fee-on-transfer / transfer-tax behavior are therefore not accepted for deposits, because the bridge refuses to mint more privacy balance than it actually receives.
