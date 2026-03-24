# Get Started

{% hint style="warning" %}
Payy Testnet and NPM packages are currently invite only - reach out to hello@payy.link for access.
{% endhint %}

Payy lets you authorise rich, wallet‑native transactions using typed signatures and execute them through the [TransactionBridge](../protocol/transactionbridge.md) with advanced features.&#x20;

You build Txn objects off‑chain with `@payy/viem`, sign them via EIP‑712, and send them through the bridge. After submission, inspect the transaction receipt and bridge status/events to confirm the outcome. Everything works with standard wallets and familiar viem primitives.

### Install

```
npm install viem @payy/viem
# or
yarn add viem @payy/viem
```

### Minimal setup

```typescript
import { createPublicClient, createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { createPayyClient, erc20 } from "@payy/viem";
import { payy } from "@payy/viem/chains";

const account = privateKeyToAccount(process.env.PRIV_KEY as `0x${string}`);

const publicClient = createPublicClient({
    transport: http(payy.rpcUrls.default.http[0]),
    chain: payy,
});

const walletClient = createWalletClient({
    account,
    transport: http(payy.rpcUrls.default.http[0]),
    chain: payy,
});

const payyClient = createPayyClient();
```

### Your first Txn: simple send

```typescript
async function simpleSend() {
    const txn = await payyClient.buildTxn
        .from(account.address)
        .calls([
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xRecipient",
                amount: 1_000_000n,
                gasLimit: 120_000n,
            }),
        ]);

    const { hash } = await payyClient.submitTxn({ txn, walletClient });
    const receipt = await publicClient.waitForTransactionReceipt({ hash });

    const txnHash = await payyClient.hashTxn({ txn, publicClient });

    console.log("Sent on-chain tx:", hash);
    console.log("Receipt status:", receipt.status);
    console.log("Txn hash:", txnHash);
}
```
