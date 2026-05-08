# Payy Client

Payy's client SDKs are the public integration surface for Payy's EVM privacy
layer:

- use `@payy/client` for TypeScript apps, wallets, and browser / Node flows
- use `payy-evm-client` for Rust backends, native apps, and Rust wallet flows

Both SDKs wrap the [`PrivacyBridge`](../../protocol/privacybridge.md) proof flows
into typed builders:

- `client.privacy().accounts()` and `client.privacy().defaultAccount()` expose signer-controlled private accounts.
- `client.privacy().notes().get(...)` resolves the latest unspent owned note for a private account and token.
- `client.privacy().balances().get(...)` returns the spendable private balance derived from the owned note.
- `client.privacy().incoming().list(...)` discovers decryptable incoming notes from `ExternalTransfer` logs.
- `client.privacy().mint(...)`, `client.privacy().burn(...)`, `client.privacy().send()`, and `client.privacy().claim()` prepare and submit proof-backed bridge transactions.

The TypeScript examples use `defaultAccount()` and camelCase fields. The Rust
examples use `default_account()` and snake_case fields, but the operation model,
network presets, prepared-call shape, and claim-link formats are the same.
Rust integrations use `bb-cli` by default on `payy-evm-client`; opt into
`bb-bindings` with `default-features = false` when you want compiled
Barretenberg bindings instead.

{% hint style="info" %}
The high-level TransactionBridge SDK namespace is reserved as `client.transactions()`, but those builders are not implemented in the current SDKs. See [Payy Transactions](../payy-transactions/README.md) for the placeholder contract-level docs.
{% endhint %}

## Guides

- [Setup](setup.md)
- [Accounts and State](accounts-and-state.md)
- [Mint, Burn, Send, Claim](private-transfers.md)
- [Adapters](adapters.md)
