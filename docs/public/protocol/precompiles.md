# Precompiles

Payy supports all standard Ethereum precompiles and adds a set of protocol-specific precompiles for privacy, proof verification, and sparse merkle tree operations.

These addresses are fixed by the `payy-evm` implementation and are the canonical precompile endpoints used by the predeployed contracts documented in [Predeployed Contracts](../build-on-payy/predeployed-contracts.md).

## Overview

| Precompile | Address | Access | Gas | Purpose |
| --- | --- | --- | --- | --- |
| Poseidon | `0x0000000000000000000000000000000000000101` | Public | `10,000 + 1,000 × element_count` | Poseidon hash over one or more 32-byte field elements |
| BlockTimestampMs | `0x0000000000000000000000000000000000000999` | Public | `2` | Returns the current block timestamp in milliseconds |
| BB Verify | `0x0000000000000000000000000000000000000998` | Public | `50,000 + 10 × calldata_bytes` | Verifies a Barretenberg proof against supplied key and public inputs |
| Privacy Proof Verify | `0x0000000000000000000000000000000000000997` | Public | `0` | Verifies supported privacy proofs and decodes transfer / burn / mint outputs |
| Native Transfer | `0x0000000000000000000000000000000000000100` | Internal-only — `PUSD` predeploy `0x0200000000000000000000000000000000000000` | `2` | Moves native balances for `PUSD` without requiring `msg.value` |
| Smirk Add | `0x0000000000000000000000000000000000000102` | Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000` | `170,000` | Inserts a leaf into the privacy layer sparse merkle tree |
| Smirk Remove | `0x0000000000000000000000000000000000000103` | Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000` | `170,000` | Removes a leaf from the privacy layer sparse merkle tree |
| Smirk Get Path | `0x0000000000000000000000000000000000000104` | Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000` | `170,000` | Returns the merkle sibling path for a leaf |
| Smirk Get Root | `0x0000000000000000000000000000000000000105` | Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000` | `10,000` | Returns the current sparse merkle tree root |

## Access model

- **Public** precompiles can be called by any contract or externally owned account.
- **Internal-only** precompiles are scoped to specific protocol predeploys and are not stable public APIs.
- `Native Transfer` returns `false` (`0x00…00`) when called by any address other than the `PUSD` predeploy at `0x0200000000000000000000000000000000000000`.
- `Smirk Add`, `Smirk Remove`, `Smirk Get Path`, and `Smirk Get Root` revert with `unauthorized` when called by any address other than the `Rollup` predeploy at `0x3200000000000000000000000000000000000000`.

## Poseidon

- **Address:** `0x0000000000000000000000000000000000000101`
- **Access:** Public
- **Canonical calldata:** raw concatenation of one or more `bytes32` field elements; no function selector or ABI array head is expected.
- **Canonical return data:** a single `bytes32` Poseidon hash.
- **Gas:** `10,000 + 1,000 × element_count`
- **Notes:** empty calldata and any calldata whose length is not a multiple of 32 bytes are rejected. The repository includes a Solidity wrapper contract that calls this precompile via `abi.encodePacked(...)`.

## BlockTimestampMs

- **Address:** `0x0000000000000000000000000000000000000999`
- **Access:** Public
- **Canonical calldata:** empty calldata. The current implementation ignores calldata and always returns the current value.
- **Canonical return data:** a single `uint256` / `bytes32` containing the block timestamp in milliseconds.
- **Gas:** `2`
- **Notes:** this supplements Ethereum's second-granularity `block.timestamp` for applications that need sub-second timing.

## BB Verify

- **Address:** `0x0000000000000000000000000000000000000998`
- **Access:** Public
- **Canonical calldata:** `abi.encode(bytes verificationKey, bytes proof, bytes publicInputs)` with no function selector.
- **Canonical return data:** ABI-encoded `bool` (`true` on successful verification, `false` on verification failure or malformed calldata).
- **Gas:** `50,000 + 10 × calldata_bytes`
- **Notes:** `publicInputs` is passed through as raw bytes to the Barretenberg verifier backend.

## Privacy Proof Verify

- **Address:** `0x0000000000000000000000000000000000000997`
- **Access:** Public
- **Canonical calldata:** `abi.encodeWithSignature("verifyPrivacyProof(bytes32,bytes,bytes32[])", verificationKeyHash, proof, publicInputs)`
- **Canonical return data:** `(uint8 kind, bytes32[] elements, uint256 value, address burnAddress)`
- **Gas:** `0`
- **Notes:** `kind` is `1` for transfer proofs, `2` for burn proofs, and `3` for mint proofs. `elements` contains the non-zero note commitments / nullifiers returned by the verifier. `value` is populated for burn and mint proofs, while `burnAddress` is only populated for burn proofs. This is the precompile consumed by [PrivacyBridge](privacybridge.md).

## Native Transfer

- **Address:** `0x0000000000000000000000000000000000000100`
- **Access:** Internal-only — `PUSD` predeploy `0x0200000000000000000000000000000000000000`
- **Canonical calldata:** `abi.encodeWithSignature("transferFromNative(address,address,uint256)", from, to, value)`
- **Canonical return data:** ABI-encoded `bool`
- **Gas:** `2`
- **Notes:** this precompile is the balance-movement primitive used by the `PUSD` predeploy for `transfer` and `transferFrom`. Unauthorized callers do not get a revert; they receive `false`.

## Smirk tree precompiles

The Smirk precompiles back the [Rollup](rollup.md) predeploy and are intentionally internal-only.

### Smirk Add

- **Address:** `0x0000000000000000000000000000000000000102`
- **Access:** Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000`
- **Canonical calldata:** either a raw `bytes32` element or `abi.encodeWithSignature("smirkAdd(bytes32)", element)`; the `Rollup` contract uses the selector-based form.
- **Canonical return data:** ABI-encoded `bool`
- **Gas:** `170,000`
- **Notes:** rejects `staticcall`, rejects the zero element, and rejects inserts for elements that already exist.

### Smirk Remove

- **Address:** `0x0000000000000000000000000000000000000103`
- **Access:** Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000`
- **Canonical calldata:** either a raw `bytes32` element or `abi.encodeWithSignature("smirkRemove(bytes32)", element)`; the `Rollup` contract uses the selector-based form.
- **Canonical return data:** ABI-encoded `bool`
- **Gas:** `170,000`
- **Notes:** rejects `staticcall`, rejects the zero element, and rejects removes for elements that do not exist.

### Smirk Get Path

- **Address:** `0x0000000000000000000000000000000000000104`
- **Access:** Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000`
- **Canonical calldata:** either a raw `bytes32` element or `abi.encodeWithSignature("smirkGetPath(bytes32)", element)`; the `Rollup` contract uses the selector-based form.
- **Canonical return data:** ABI-encoded `bytes32[]` merkle siblings. With the current tree depth of 161, the returned array contains 160 sibling hashes.
- **Gas:** `170,000`

### Smirk Get Root

- **Address:** `0x0000000000000000000000000000000000000105`
- **Access:** Internal-only — `Rollup` predeploy `0x3200000000000000000000000000000000000000`
- **Canonical calldata:** empty calldata or `abi.encodeWithSignature("smirkGetRoot()")`
- **Canonical return data:** a single `bytes32` sparse merkle tree root.
- **Gas:** `10,000`

## Related docs

- [Predeployed Contracts](../build-on-payy/predeployed-contracts.md)
- [PrivacyBridge](privacybridge.md)
- [Rollup](rollup.md)
- [EVM Layer](evm-layer.md)
