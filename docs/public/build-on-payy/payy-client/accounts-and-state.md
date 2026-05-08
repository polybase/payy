# Accounts and State

## Private Accounts

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const accounts = await client.privacy().accounts();
const privacyAccount = await client.privacy().defaultAccount();
```
{% endtab %}

{% tab title="Rust" %}
```rust
let accounts = client.privacy().accounts()?;
let privacy_account = client
    .privacy()
    .default_account()?
    .ok_or(AppError::MissingPayyPrivacyAccount)?;
```
{% endtab %}

{% endtabs %}

Private accounts select Payy private addresses controlled by the configured privacy signer. They are not EVM accounts.

## Owned Notes and Checkpoints

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const state = await client.privacy().notes().get({
  privacyAccount,
  token,
});

await client.setCheckpoint(state);

const nextState = await client
  .privacy()
  .notes()
  .withCheckpoint(state)
  .get({ privacyAccount, token });
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::OwnedNoteGetParams;

let state = client
    .privacy()
    .notes()
    .get(OwnedNoteGetParams {
        privacy_account: privacy_account.clone(),
        token,
    })
    .await?;

client.set_checkpoint(state.clone())?;

let next_state = client
    .privacy()
    .notes()
    .with_checkpoint(state)
    .get(OwnedNoteGetParams {
        privacy_account: privacy_account.clone(),
        token,
    })
    .await?;
```
{% endtab %}

{% endtabs %}

`notes().get(...)` returns the latest unspent owned note plus the highest checked block. Callers can persist that state and seed it back through `setCheckpoint(...)` or `withCheckpoint(...)` to avoid a full lookup on the next process start. The client still validates the checkpoint against chain state before returning or spending it.

## Balances

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const balance = await client.privacy().balances().get({
  privacyAccount,
  token,
});

console.log(balance.balance?.spendable ?? 0n);
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::OwnedNoteGetParams;

let balance = client
    .privacy()
    .balances()
    .get(OwnedNoteGetParams {
        privacy_account: privacy_account.clone(),
        token,
    })
    .await?;

let spendable = balance.balance.as_ref().map(|balance| balance.spendable);
```
{% endtab %}

{% endtabs %}

Private balances are derived from the latest unspent owned note.

## Incoming Discovery

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const notes = await client.privacy().incoming().list({
  privacyAccount,
  fromBlock: 0n,
  includeSpent: false,
});
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::IncomingListParams;

let notes = client
    .privacy()
    .incoming()
    .list(IncomingListParams {
        privacy_account: privacy_account.clone(),
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

`incoming().list(...)` scans `ExternalTransfer(prefix6, txHash)` logs, decrypts candidate recipient notes, and skips spent notes by default. Set `includeSpent: true` to include spent notes with status.

## Watch Resume

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const result = await client.privacy().incoming().watch(
  {
    privacyAccount,
    fromBlock: savedNextFromBlock,
    includeSpent: false,
    pollIntervalMs: 3_000,
  },
  async (note) => {
    await persistIncomingNote(note);
  }
);

await persistNextFromBlock(result.nextFromBlock);
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::{IncomingListParams, Result};

let result = client
    .privacy()
    .incoming()
    .watch(
        IncomingListParams {
            privacy_account,
            privacy_address_prefix: None,
            from_block: saved_next_from_block,
            to_block: None,
            include_spent: false,
            poll_interval_ms: Some(3_000),
        },
        |note| -> Result<()> {
            persist_incoming_note(note)?;
            Ok(())
        },
    )
    .await?;

persist_next_from_block(result.next_from_block)?;
```
{% endtab %}

{% endtabs %}

`watch(...)` replays a block until every callback for that block completes. Resume from `nextFromBlock`.
