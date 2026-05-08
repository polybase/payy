# Adapters

Payy's SDKs define small adapter interfaces instead of depending on a single EVM
client stack. TypeScript ships helper adapters for viem and ethers. Rust accepts
trait objects and ships first-party Alloy helpers in `payy-evm-client-alloy` or the
`payy_evm_client` root when its `alloy` feature is enabled.

{% tabs %}

{% tab title="viem" %}
### Direct wallet submission

```typescript
import { createPayyClient } from "@payy/client";
import {
  toViemTransaction,
  viemPublicClientAdapter,
} from "@payy/client/viem";

const client = createPayyClient({
  publicClient: viemPublicClientAdapter(publicClient),
}).privacySigner(myPrivacySigner);

const prepared = await client
  .privacy()
  .mint({
    privacyAccount,
    evmAccount: account.address,
    token,
    amount,
  })
  .prepare();

const hash = await walletClient.sendTransaction(
  toViemTransaction(prepared),
);
```

### SDK local raw submission

```typescript
import { createPayyClient } from "@payy/client";
import {
  viemPublicClientAdapter,
  viemRawTransactionSubmitter,
} from "@payy/client/viem";

const client = createPayyClient({
  publicClient: viemPublicClientAdapter(publicClient),
  rawTransactionSubmitter: viemRawTransactionSubmitter(publicClient),
}).withEvmPrivateKey(process.env.PRIV_KEY as `0x${string}`);

const prepared = await client
  .privacy()
  .mint({
    privacyAccount,
    evmAccount: account.address,
    token,
    amount,
  })
  .prepare();

const submitted = await prepared.submit();
```

- `viemPublicClientAdapter(publicClient)` adapts a viem public client for reads, logs, receipts, and receipt waiting.
- `viemWalletSubmitter(walletClient)` delegates transaction submission to the wallet account.
- `viemRawTransactionSubmitter(publicClient)` lets `withEvmPrivateKey(...)` locally sign and broadcast raw EIP-1559 transactions.
- `toViemTransaction(prepared, { chain?, account? })` converts a prepared Payy call into a viem transaction request for direct `walletClient.sendTransaction(...)` usage. Omit `chain` when the wallet client is already configured with the Payy chain; pass it only for an explicit conversion-time chain check.
{% endtab %}

{% tab title="ethers" %}
### Direct signer submission

```typescript
import { createPayyClient } from "@payy/client";
import {
  ethersProviderAdapter,
  toEthersTransaction,
} from "@payy/client/ethers";

const client = createPayyClient({
  publicClient: ethersProviderAdapter(provider),
}).privacySigner(myPrivacySigner);

const signerAddress = (await signer.getAddress()) as `0x${string}`;
const prepared = await client
  .privacy()
  .mint({
    privacyAccount,
    evmAccount: signerAddress,
    token,
    amount,
  })
  .prepare();

const response = await signer.sendTransaction(toEthersTransaction(prepared));
```

### SDK local raw submission

```typescript
import { createPayyClient } from "@payy/client";
import {
  ethersProviderAdapter,
  ethersRawTransactionSubmitter,
} from "@payy/client/ethers";

const client = createPayyClient({
  publicClient: ethersProviderAdapter(provider),
  rawTransactionSubmitter: ethersRawTransactionSubmitter(provider),
}).withEvmPrivateKey(process.env.PRIV_KEY as `0x${string}`);

const prepared = await client
  .privacy()
  .mint({
    privacyAccount,
    evmAccount: account.address,
    token,
    amount,
  })
  .prepare();

const submitted = await prepared.submit();
```

- `ethersProviderAdapter(provider)` adapts an ethers provider for reads, logs, receipts, and receipt waiting.
- `ethersSignerSubmitter(signer)` delegates transaction submission to an ethers signer.
- `ethersRawTransactionSubmitter(provider)` lets `withEvmPrivateKey(...)` locally sign and broadcast raw EIP-1559 transactions.
- `toEthersTransaction(prepared, { chainId?, from? })` converts a prepared Payy call into an ethers transaction request for direct `signer.sendTransaction(...)` usage.
{% endtab %}

{% tab title="Rust" %}
### Alloy helpers

```rust
use payy_evm_client::{
    alloy_raw_transaction_submitter, alloy_read_client, alloy_wallet_submitter,
    alloy_wallet_submitter_with_address, to_alloy_transaction, BaseClient, PayyNetworkPreset,
};
use alloy::providers::Provider;

let base_client = BaseClient::builder(
    PayyNetworkPreset::Testnet.config(),
    alloy_read_client(provider.clone()),
)
    .raw_transaction_submitter(alloy_raw_transaction_submitter(provider.clone()))
    .build();

let delegated_client = base_client
    .clone()
    .with_grumpkin_private_key(grumpkin_private_key)?
    .evm_signer(alloy_wallet_submitter_with_address(
        wallet_provider,
        wallet_address,
    ));

let local_client = base_client.with_evm_private_key(evm_private_key)?;

let prepared = local_client.privacy().mint(params).prepare().await?;
let pending = provider
    .send_transaction(to_alloy_transaction(&prepared)?)
    .await?;
```

- `PayyEvmReadClient` provides chain ID, block number, `eth_call`, log reads, receipt lookup, and receipt waiting.
- `PayyEvmSubmitter` delegates a prepared bridge transaction to an external wallet or signer.
- `PayyRawTransactionSubmitter` lets `with_evm_private_key(...)` locally sign and broadcast raw EIP-1559 transactions.
- `alloy_read_client(provider)` adapts an Alloy provider for reads, logs, receipts, and receipt waiting.
- `alloy_wallet_submitter(provider)` delegates transaction submission and infers the sender from the provider's first account.
- `alloy_wallet_submitter_with_address(provider, address)` delegates transaction submission to an Alloy wallet provider and validates the expected sender.
- `alloy_raw_transaction_submitter(provider)` lets `with_evm_private_key(...)` locally sign and broadcast raw EIP-1559 transactions through an Alloy provider.
- `to_alloy_transaction(prepared)` converts a prepared Payy call into an Alloy transaction request for direct `provider.send_transaction(...)` usage.
- `raw_transaction_submitter(...)` is optional for Rust prepare-only flows. Without it, `with_evm_private_key(...)` still derives the privacy signer, but SDK-owned submission requires either `evm_signer(...)`, an `EvmAccount::Signer`, or a raw transaction submitter.
{% endtab %}

{% endtabs %}

For `mint(...)`, `evmAccount` must match the EVM signer that submits the transaction because the bridge enforces `mint_from == msg.sender`.
