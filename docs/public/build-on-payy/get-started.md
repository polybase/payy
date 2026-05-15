# Get Started

{% hint style="warning" %}
Payy Testnet and SDK packages are in alpha - you should expect breaking changes.
{% endhint %}

Use Payy's client SDKs for EVM privacy flows: private accounts, owned-note lookup, private balances, incoming-note discovery, proof preparation, and PrivacyBridge submission.

For the full SDK guide, start with [Payy Client](payy-client/README.md).

## Install

{% tabs %}

{% tab title="viem" %}
```bash
npm install @payy/client viem
# or
yarn add @payy/client viem
```
{% endtab %}

{% tab title="ethers" %}
```bash
npm install @payy/client ethers
# or
yarn add @payy/client ethers
```
{% endtab %}

{% tab title="Rust" %}
```toml
# Once crates.io publishing is enabled:
payy-evm-client = { version = "0.1", features = ["alloy"] }

# Until then, pin the repo revision:
payy-evm-client = { git = "https://github.com/polybase/payy", package = "payy-evm-client", rev = "<commit>", features = ["alloy"] }
```

Rust builds use the `bb-cli` backend by default and shell out to `bb` version
`3.0.0-manual.20251030` on `PATH`. Install that version with `bbup`:

```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/master/barretenberg/bbup/install | bash
bbup -v 3.0.0-manual.20251030
bb --version
```

To use compiled Barretenberg bindings instead, enable `bb-bindings`. Cargo
features are additive, so add `default-features = false` only if you also want to
omit the default `bb-cli` dependency.
{% endtab %}

{% endtabs %}

## Minimal Setup

{% tabs %}

{% tab title="viem" %}
```typescript
import { createPayyClient } from "@payy/client";
import {
  chains,
  toViemTransaction,
  viemPublicClientAdapter,
} from "@payy/client/viem";
import { createPublicClient, createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";

const evmPrivateKey = process.env.PRIV_KEY as `0x${string}`;
const payyChain = chains.payy.testnet;
const account = privateKeyToAccount(evmPrivateKey);

const publicClient = createPublicClient({
  chain: payyChain,
  transport: http(),
});

const walletClient = createWalletClient({
  account,
  chain: payyChain,
  transport: http(),
});

const client = createPayyClient({
  publicClient: viemPublicClientAdapter(publicClient),
}).withEvmPrivateKey(evmPrivateKey);

const privacyAccount = await client.privacy().defaultAccount();
if (privacyAccount === null) {
  throw new Error("missing Payy privacy account");
}

const prepared = await client
  .privacy()
  .mint({
    privacyAccount,
    evmAccount: account.address,
    token: process.env.TOKEN_ADDRESS as `0x${string}`,
    amount: 1_000_000n,
  })
  .prepare();

const hash = await walletClient.sendTransaction(
  toViemTransaction(prepared),
);
```
{% endtab %}

{% tab title="ethers" %}
```typescript
import { createPayyClient } from "@payy/client";
import {
  ethersProviderAdapter,
  toEthersTransaction,
} from "@payy/client/ethers";
import { JsonRpcProvider, Wallet } from "ethers";

const evmPrivateKey = process.env.PRIV_KEY as `0x${string}`;
const provider = new JsonRpcProvider(process.env.PAYY_RPC_URL);
const wallet = new Wallet(evmPrivateKey, provider);
const evmAccount = (await wallet.getAddress()) as `0x${string}`;

const client = createPayyClient({
  publicClient: ethersProviderAdapter(provider),
}).withEvmPrivateKey(evmPrivateKey);

const privacyAccount = await client.privacy().defaultAccount();
if (privacyAccount === null) {
  throw new Error("missing Payy privacy account");
}

const prepared = await client
  .privacy()
  .mint({
    privacyAccount,
    evmAccount,
    token: process.env.TOKEN_ADDRESS as `0x${string}`,
    amount: 1_000_000n,
  })
  .prepare();

const response = await wallet.sendTransaction(toEthersTransaction(prepared));
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::{
    alloy_read_client, to_alloy_transaction, BaseClient, EvmAccount, MintParams,
    PayyNetworkPreset,
};
use alloy::providers::Provider;

let client = BaseClient::builder(
    PayyNetworkPreset::Testnet.config(),
    alloy_read_client(provider.clone()),
)
    .build()
    .with_evm_private_key(evm_private_key)?;

let privacy_account = client
    .privacy()
    .default_account()?
    .ok_or(AppError::MissingPayyPrivacyAccount)?;

let prepared = client
    .privacy()
    .mint(MintParams {
        privacy_account,
        evm_account: EvmAccount::Address(evm_account),
        token,
        amount: 1_000_000u64.into(),
    })
    .prepare()
    .await?;

let pending = provider
    .send_transaction(to_alloy_transaction(&prepared)?)
    .await?;
```
{% endtab %}

{% endtabs %}

`privacyAccount` selects a Payy private account / private address. `evmAccount` selects the public EVM sender used for operations such as `mint`, where the bridge requires `mint_from == msg.sender`.

The TypeScript and Rust clients expose the same privacy operations. TypeScript uses
camelCase object fields and viem / ethers conversion helpers; Rust uses snake_case
params structs, EVM adapter traits, and `to_alloy_transaction(...)` when using the
first-party Alloy adapter.
