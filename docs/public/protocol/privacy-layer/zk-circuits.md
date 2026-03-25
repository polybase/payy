# ZK Circuits

The [PrivacyBridge](../privacybridge.md) interface methods accept the following ZK circuits as proofs:

- [`transfer`](https://github.com/polybase/payy/tree/main/noir/evm/transfer) - internal transfer within the privacy pool
- [`burn`](https://github.com/polybase/payy/tree/main/noir/evm/burn) - withdraw from the privacy pool
- [`mint`](https://github.com/polybase/payy/tree/main/noir/evm/mint) - deposit into the privacy pool
- [`erc20_transfer`](https://github.com/polybase/payy/tree/main/noir/evm/erc20_transfer) - ERC-20 transfer proof (transparent upgrade using an standard ERC-20 transfer signature)

{% include "../../../../.gitbook/includes/zk-framework.md" %}

## Manual proof construction

When using the [@payy/client](../../build-on-payy/get-started.md), the client will construct the proofs for you. If you are constructing PrivacyBridge ZK proofs client-side without the Payy SDK, you must use [`@aztec/bb.js` version `3.0.0-manual.20251030`](https://www.npmjs.com/package/@aztec/bb.js/v/3.0.0-manual.20251030) for manual proof generation, with the above ZK circuits.
