# Private Transfers

Payy private transfers are built on top of [`PrivacyBridge`](../protocol/privacybridge.md) and exposed through the Payy client SDKs. Use [`@payy/client`](payy-client/README.md) for TypeScript integrations and `payy-evm-client` for Rust integrations.

The protocol model is:

1. `mint` deposits ERC-20 value into a private owned note.
2. `transfer_send` spends an owned note and creates a sender continuation note plus a recipient incoming note.
3. `ExternalTransfer(prefix6, txHash)` lets recipients discover decryptable incoming notes.
4. `transfer_claim` merges an incoming note into the recipient's owned-note chain.
5. `burn` withdraws private value back to a public EVM recipient.

{% hint style="info" %}
All private token transfers are gas zero rated to enable [zero fee private payments](../stablecoins/zero-fee-payments.md).
{% endhint %}

## SDK Mapping

| Protocol concept | SDK surface |
| --- | --- |
| Private address / `PrivacyAddress` | `PrivacyAccount` selector |
| Owned note-chain state | `client.privacy().notes().get(...)` |
| Private balance | `client.privacy().balances().get(...)` |
| `ExternalTransfer(prefix6, txHash)` | `client.privacy().incoming().list(...)` / `watch(...)` |
| `transfer_send` | `client.privacy().send().to(...)` or `client.privacy().send().ephemeral(...)` |
| `transfer_claim` | `client.privacy().claim().account(...).note(...)`, `.link(...)`, or `.ephemeral(...)` |

## Recipient Private Account

Private transfers do not route by EVM address alone. The recipient shares a Payy private address, represented in the SDK as a `PrivacyAccount` or `PrivacyAddress`.

The sender does not need the recipient's current wallet `psi`. Direct sends create incoming notes with `nonce = 0`; the recipient later claims them into their own wallet chain.

## Discovery

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const incoming = await client.privacy().incoming().list({
  privacyAccount,
  fromBlock: 0n,
  includeSpent: false,
});
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::IncomingListParams;

let incoming = client
    .privacy()
    .incoming()
    .list(IncomingListParams {
        privacy_account,
        privacy_address_prefix: None,
        from_block: 0,
        to_block: None,
        include_spent: false,
        poll_interval_ms: None,
    })
    .await?;
```
{% endtab %}

{% endtabs %}

The client filters `ExternalTransfer` logs by the recipient prefix, fetches `TxnData`, decrypts candidate recipient notes, and checks nullifier status.

## Links and Messages

Direct-send links carry claim metadata and an optional link `message`.

The link `message` is off-chain link metadata. It is separate from the bridge `memo`, which is an on-chain `bytes32` field on `transfer_send`.

## Transparent Wallet Compatibility

Existing wallet UX can still be supported by an RPC or privacy service layer, but that layer must know the recipient's private address in addition to the normal transfer intent.

No off-chain inbox publish step is required for the standard protocol flow.
