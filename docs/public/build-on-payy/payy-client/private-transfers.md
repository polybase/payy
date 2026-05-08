# Mint, Burn, Send, Claim

## Mint

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const prepared = await client
  .privacy()
  .mint({
    privacyAccount,
    evmAccount, // public EVM sender; must match msg.sender
    token,
    amount,
  })
  .prepare();

const confirmed = await prepared.submitAndWait();
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::{EvmAccount, MintParams};

let prepared = client
    .privacy()
    .mint(MintParams {
        privacy_account: privacy_account.clone(),
        evm_account: EvmAccount::Address(evm_account),
        token,
        amount,
    })
    .prepare()
    .await?;

let confirmed = prepared.submit_and_wait().await?;
```
{% endtab %}

{% endtabs %}

## Burn

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const state = await client.privacy().notes().get({ privacyAccount, token });

const confirmed = await client
  .privacy()
  .burn({
    privacyAccount,
    token,
    amount,
    evmRecipient,
  })
  .withCheckpoint(state)
  .prepare()
  .then((prepared) => prepared.submitAndWait());
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::{BurnParams, OwnedNoteGetParams};

let state = client
    .privacy()
    .notes()
    .get(OwnedNoteGetParams {
        privacy_account: privacy_account.clone(),
        token,
    })
    .await?;

let confirmed = client
    .privacy()
    .burn(BurnParams {
        privacy_account: privacy_account.clone(),
        token,
        amount,
        evm_recipient,
    })
    .with_checkpoint(state)
    .prepare()
    .await?
    .submit_and_wait()
    .await?;
```
{% endtab %}

{% endtabs %}

## Direct Private Send

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const state = await client.privacy().notes().get({ privacyAccount, token });

const prepared = await client
  .privacy()
  .send()
  .to({
    privacyAccount,
    token,
    amount,
    recipient: recipientPrivateAddress,
    bridgeMemo: "0x0000000000000000000000000000000000000000000000000000000000000000",
  })
  .withCheckpoint(state)
  .prepare();

const confirmed = await prepared.submitAndWait();
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::{DirectSendParams, OwnedNoteGetParams};

let state = client
    .privacy()
    .notes()
    .get(OwnedNoteGetParams {
        privacy_account: privacy_account.clone(),
        token,
    })
    .await?;

let prepared = client
    .privacy()
    .send()
    .to(DirectSendParams {
        privacy_account: privacy_account.clone(),
        token,
        amount,
        recipient: recipient_privacy_address,
        bridge_memo: Some([0u8; 32]),
    })
    .with_checkpoint(state)
    .prepare()
    .await?;

let confirmed = prepared.submit_and_wait().await?;
```
{% endtab %}

{% endtabs %}

`bridgeMemo` is the on-chain `bytes32 memo` carried by `transfer_send`.

## Claim Link Message

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const prepared = await client
  .privacy()
  .send()
  .to({
    privacyAccount,
    token,
    amount,
    recipient: recipientPrivateAddress,
  })
  .withCheckpoint(state)
  .link("Dinner");

const [delivery, claimLink] = prepared.result.payload;
await prepared.submitAndWait();
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::DirectSendParams;

let prepared = client
    .privacy()
    .send()
    .to(DirectSendParams {
        privacy_account: privacy_account.clone(),
        token,
        amount,
        recipient: recipient_privacy_address,
        bridge_memo: None,
    })
    .with_checkpoint(state)
    .link(Some("Dinner"))
    .await?;

let (delivery, claim_link) = prepared.payload();
prepared.submit_and_wait().await?;
```
{% endtab %}

{% endtabs %}

The link `message` is off-chain link metadata. It is distinct from the bridge `memo`.

## Ephemeral Handoff

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const prepared = await client
  .privacy()
  .send()
  .ephemeral({
    privacyAccount,
    token,
    amount,
  })
  .withCheckpoint(state)
  .prepare();

const incomingTransfer = prepared.result.payload;
await prepared.submitAndWait();
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::EphemeralSendParams;

let prepared = client
    .privacy()
    .send()
    .ephemeral(EphemeralSendParams {
        privacy_account: privacy_account.clone(),
        token,
        amount,
        bridge_memo: None,
    })
    .with_checkpoint(state)
    .prepare()
    .await?;

let incoming_transfer = prepared.payload().clone();
prepared.submit_and_wait().await?;
```
{% endtab %}

{% endtabs %}

`IncomingTransfer` is a bearer artifact. Store and transmit it as claimable secret material.

## Claims

{% tabs %}

{% tab title="TypeScript" %}
```typescript
const directClaim = await client
  .privacy()
  .claim()
  .account(privacyAccount)
  .note(incomingNote)
  .prepare();

await directClaim.submitAndWait();

const linkClaim = await client
  .privacy()
  .claim()
  .account(privacyAccount)
  .link(claimLink)
  .prepare();

await linkClaim.submitAndWait();

const ephemeralClaim = await client
  .privacy()
  .claim()
  .account(privacyAccount)
  .ephemeral(incomingTransfer)
  .prepare();

await ephemeralClaim.submitAndWait();
```
{% endtab %}

{% tab title="Rust" %}
```rust
let direct_claim = client
    .privacy()
    .claim()
    .account(privacy_account.clone())
    .note(incoming_note)
    .prepare()
    .await?;

direct_claim.submit_and_wait().await?;

let link_claim = client
    .privacy()
    .claim()
    .account(privacy_account.clone())
    .link(claim_link)
    .prepare()
    .await?;

link_claim.submit_and_wait().await?;

let ephemeral_claim = client
    .privacy()
    .claim()
    .account(privacy_account)
    .ephemeral(incoming_transfer)
    .prepare()
    .await?;

ephemeral_claim.submit_and_wait().await?;
```
{% endtab %}

{% endtabs %}
