# Setup

## Install

{% tabs %}

{% tab title="viem" %}
```bash
yarn add @payy/client viem
```

`@payy/client` installs the supported `@aztec/bb.js@3.0.0-manual.20251030` proving backend automatically. Install `viem` separately when using the viem adapter helpers.
{% endtab %}

{% tab title="ethers" %}
```bash
yarn add @payy/client ethers
```

`@payy/client` installs the supported `@aztec/bb.js@3.0.0-manual.20251030` proving backend automatically. Install `ethers` separately when using the ethers adapter helpers.
{% endtab %}

{% tab title="Rust" %}
```toml
# Once crates.io publishing is enabled:
payy-evm-client = { version = "0.1", features = ["alloy"] }

# Until then, pin the repo revision:
payy-evm-client = { git = "https://github.com/polybase/payy", package = "payy-evm-client", rev = "<commit>", features = ["alloy"] }

# Or depend on the adapter crate explicitly:
payy-evm-client-alloy = { git = "https://github.com/polybase/payy", package = "payy-evm-client-alloy", rev = "<commit>" }
```

The Rust client enables `bb-cli` by default, so it shells out to a `bb`
executable on `PATH`. Use `default-features = false` with `bb-bindings` if you
want compiled Barretenberg bindings instead. Enable the `alloy` feature for
first-party Alloy helpers, or depend on `payy-evm-client-alloy` directly if you
prefer explicit crate imports.
{% endtab %}

{% endtabs %}

## Client Construction

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

const baseClient = createPayyClient({
  publicClient: viemPublicClientAdapter(publicClient),
});

const client = baseClient.withEvmPrivateKey(evmPrivateKey);

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

let base_client = BaseClient::builder(
    PayyNetworkPreset::Testnet.config(),
    alloy_read_client(provider.clone()),
)
.build();

let client = base_client.with_evm_private_key(evm_private_key)?;

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

A read-only base client can parse links and read bridge state. A privacy-capable client can discover notes and prepare proofs. Submit through an SDK submitter with `submit()` / `submitAndWait()` in TypeScript or `submit()` / `submit_and_wait()` in Rust, or convert / extract the prepared bridge request for native wallet submission.

`createPayyClient` reads the chain ID from the public-client adapter when preparing operations. `toViemTransaction` converts the prepared bridge request into the shape expected by `walletClient.sendTransaction(...)`; pass `{ chain }` only when you want the helper to preflight-check the prepared chain ID before viem handles the configured wallet chain. The client uses Payy's default PrivacyBridge address; pass `privacyBridge` only when connecting to a custom bridge deployment.

`BaseClient::builder(...)` takes the explicit `PayyNetworkConfig` plus a read adapter. The Rust client validates the adapter chain ID against that config before bridge reads and prepared privacy operations.

Rust `with_evm_private_key(...)` does not require `raw_transaction_submitter(...)`
when you only need privacy signing and `prepare()`. Configure a raw transaction
submitter only when you want `prepared.submit()` / `submit_and_wait()` to locally
sign and broadcast through the SDK.

Rust local privacy signing and the default prover use the `bb-cli` feature by
default. Use `bb-bindings` with `default-features = false` when you need the
compiled Barretenberg binding backend.

## Local EVM Key

{% tabs %}

{% tab title="TypeScript" %}
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
```
{% endtab %}

{% tab title="Rust" %}
```rust
use payy_evm_client::{
    alloy_raw_transaction_submitter, alloy_read_client, BaseClient, PayyNetworkPreset,
};

let client = BaseClient::builder(
    PayyNetworkPreset::Testnet.config(),
    alloy_read_client(provider.clone()),
)
    .raw_transaction_submitter(alloy_raw_transaction_submitter(provider.clone()))
    .build()
    .with_evm_private_key(evm_private_key)?;
```
{% endtab %}

{% endtabs %}

`withEvmPrivateKey(evmPrivateKey)` derives both the EVM signer identity and the local privacy signer from the supplied secp256k1 key. With a raw-transaction submitter configured, it can also submit locally signed EVM transactions.

`withSecp256k1PrivateKey(evmPrivateKey)` / `with_secp256k1_private_key(evm_private_key)` is an explicit alias for the same EVM-key path.

`withGrumpkinPrivateKey(grumpkinPrivateKey)` / `with_grumpkin_private_key(grumpkin_private_key)` configures only the local privacy signer. It does not add local EVM submission because a Grumpkin key cannot derive an EVM sender.

`privacyAccount` is the private-account selector. `evmAccount` is the public EVM sender / signer selector.
