# ZK Circuits

The [PrivacyBridge](../privacybridge.md) interface accepts the following ZK circuits as proofs:

- [`mint`](https://github.com/polybase/payy/tree/main/noir/evm/mint) via `PrivacyBridge.mint(...)` for deposits into the privacy pool
- [`burn`](https://github.com/polybase/payy/tree/main/noir/evm/burn) via `PrivacyBridge.burn(...)` for withdrawals out of the privacy pool
- [`transfer_send`](https://github.com/polybase/payy/tree/main/noir/evm/transfer_send) via `PrivacyBridge.transfer(...)` for sender-side direct private transfers
- [`transfer_claim`](https://github.com/polybase/payy/tree/main/noir/evm/transfer_claim) via `PrivacyBridge.transfer(...)` for recipient-side claim / merge of incoming notes
- [`erc20_transfer`](https://github.com/polybase/payy/tree/main/noir/evm/erc20_transfer) for transparent ERC-20 transfer upgrades

`transfer_send` and `transfer_claim` share the same bridge entrypoint. The proof verifier returns a
distinct kind for each transfer circuit variant, and the bridge uses that decoded kind for transfer-side
branching and calldata validation, while both still use the same canonical 33-field public input layout.

{% include "../../../../.gitbook/includes/zk-framework.md" %}

## Manual proof construction

When using the [Payy client SDKs](../../build-on-payy/payy-client/README.md), the client constructs the proofs for you. If you are manually constructing PrivacyBridge ZK proofs in TypeScript outside the Payy SDK, use [`@aztec/bb.js` version `3.0.0-manual.20251030`](https://www.npmjs.com/package/@aztec/bb.js/v/3.0.0-manual.20251030). Rust integrations use the Rust SDK prover. Manual proof callers must match the above ZK circuits and the current bridge calldata shape:

- every privacy call carries `userEncryptedKey`
- `transfer_send` additionally carries `recipientEncryptedKey` and `bytes32 memo`
- non-send variants zero the recipient-side public input slots
