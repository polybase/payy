# @payy/client

TypeScript SDK for Payy's EVM privacy bridge.

`@payy/client` provides the TypeScript client surface, first-party viem and
ethers adapter helpers, local privacy signing, and proof generation integration
for Payy private transfers.

For complete setup, state, transfer, and adapter guides, see the
[Payy Client docs](https://docs.payy.network/build-on-payy/payy-client).

## Install

With viem:

```bash
yarn add @payy/client viem
```

With ethers:

```bash
yarn add @payy/client ethers
```

`@payy/client` installs the supported `@aztec/bb.js` proving backend
automatically. Install `viem` or `ethers` separately when using those adapter
helpers.

## viem

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
const account = privateKeyToAccount(evmPrivateKey);
const chain = chains.payy.testnet;

const publicClient = createPublicClient({
  chain,
  transport: http(),
});

const walletClient = createWalletClient({
  account,
  chain,
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

const hash = await walletClient.sendTransaction(toViemTransaction(prepared));
```

## ethers

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

## Client Behavior

`createPayyClient` accepts read/submit adapters. Chain IDs are read from those
adapters when preparing or submitting operations. The PrivacyBridge address
defaults to Payy's standard deployment and can be overridden with the
`privacyBridge` option for custom deployments.

Use `withEvmPrivateKey(evmPrivateKey)` when the app should derive both the EVM
sender identity and local privacy signer from a secp256k1 key. Use
`withGrumpkinPrivateKey(grumpkinPrivateKey)` when the app should configure only
the privacy signer.

Privacy operations live under `client.privacy()`, including `mint(...)`,
`burn(...)`, `send()`, `claim()`, `notes()`, `balances()`, and `incoming()`.
Prepared operations can be submitted through SDK submitters or converted to
native wallet transaction requests with `toViemTransaction(...)` and
`toEthersTransaction(...)`.
