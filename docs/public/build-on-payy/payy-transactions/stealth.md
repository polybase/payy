# Stealth

Stealth mode lets you execute calls on the [EVM Layer](../../protocol/evm-layer.md) without linking them to your public EOA. Instead of spending directly from your address, Payy temporarily funds a fresh, one-time address using private tokens from your [Privacy Vault](../../protocol/privacy-vault.md), executes your calls, then returns any remaining funds back to the [Privacy Layer](../../protocol/privacy-layer/) when finished.

You can enable it with:

* **`.stealth()`** — generate a new one-time address automatically.
* **`.stealthAs(pk: string)`** — use a specific private key for the one-time address (advanced / power-users).

{% hint style="info" %}
Because Payy uses [zero rates all private transfers](../../stablecoins/zero-fee-payments.md), moving funds to the one-time address does not incur any gas fees.
{% endhint %}

### Example

```typescript
import { createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import {
  erc20,
  createPayyClient,
  // dex, // (placeholder) swap helper(s) depending on your SDK
} from "@payy/viem";
import { payy } from "@payy/viem/chains";

const account = privateKeyToAccount(process.env.PRIV_KEY as `0x${string}`);

const walletClient = createWalletClient({
  account,
  transport: http(payy.rpcUrls.default.http[0]),
  chain: payy,
});

const payyClient = createPayyClient();

async function stealthSwap() {
  const tokenIn = "0xTokenIn";
  const tokenOut = "0xTokenOut";
  const router = "0xDexRouter"; // e.g. UniswapV2/V3 router address
  const amountIn = 1_000_000n;
  const minAmountOut = 990_000n; // slippage protection
  const recipient = "0xRecipient"; // could be your EOA or another address

  const txn = await payyClient.buildTxn
    .stealth()
    .calls([
      // 1) Approve router to spend tokenIn (from the stealth address’ balance)
      erc20.approve({
        token: tokenIn,
        spender: router,
        amount: amountIn,
        gasLimit: 60_000n,
      }),

      // 2) Swap tokenIn -> tokenOut
      // Replace this call with the swap helper your SDK exposes.
      // The important part: it runs inside the same stealth-funded Txn.
      {
        to: router,
        data: "0x...", // encoded swap calldata
        value: 0n,
        gasLimit: 250_000n,
      },
    ])
    .requireSuccess(true); // atomic: approve+swap must both succeed

  const { hash: txHash } = await payyClient.submitTxn({
    txn,
    walletClient,
  });

  console.log("Sent stealth swap tx:", txHash);
}
```

### Manual Stealth Transaction Example

Using the [Payy Transaction](../../stablecoins/payy-transactions.md) primitives, you may construct a Stealth Transaction manually if you prefer more control over the movement of funds.

```typescript
import { createWalletClient, http, createPublicClient } from "viem";
import { privateKeyToAccount, generatePrivateKey, privateKeyToAddress } from "viem/accounts";
import {
    erc20,
    privacyBridge,
    createPayyClient,
} from "@payy/viem";
import { payy } from "@payy/viem/chains";

// Origin (real) wallet
const originAccount = privateKeyToAccount(process.env.PRIV_KEY as `0x${string}`);

const walletClient = createWalletClient({
    account: originAccount,
    transport: http(payy.rpcUrls.default.http[0]),
    chain: payy,
});

const publicClient = createPublicClient({
    transport: http(payy.rpcUrls.default.http[0]),
    chain: payy,
});

const payyClient = createPayyClient();

// Ephemeral (stealth) wallet
const ephemeralPrivKey = generatePrivateKey();
const ephemeralAddress = privateKeyToAddress(ephemeralPrivKey);
const ephemeralAccount = privateKeyToAccount(ephemeralPrivKey);

async function stealthTransactionSingleBatch() {
    const TOKEN = "0xYourERC20" as `0x${string}`;
    const GAS_LIMIT = 80_000n;

    // Plan: fund -> two transfers as ephemeral -> sweep back to origin
    const txn = await payyClient.buildTxn
        .calls([
            // 1) Fund ephemeral with ERC‑20 from origin
            erc20.transfer({
                token: TOKEN,
                to: ephemeralAddress,
                amount: 3_500_000n,
                gasLimit: GAS_LIMIT,
            }).as(originAccount),

            // 2) Spend as the ephemeral wallet
            erc20.transfer({
                token: TOKEN,
                to: "0xAlice",
                amount: 1_000_000n,
                gasLimit: GAS_LIMIT,
            }).as(ephemeralAccount),

            erc20.transfer({
                token: TOKEN,
                to: "0xBob",
                amount: 500_000n,
                gasLimit: GAS_LIMIT,
            }).as(ephemeralAccount),

            // 3) Sweep remaining funds back to origin from the ephemeral wallet
            privacyBridge.sweep({
                token: TOKEN,
                to: originAccount.address,
                gasLimit: GAS_LIMIT,
            }).as(ephemeralAccount),
        ])
        // Atomic by default; if any step fails, everything reverts
        .requireSuccess(true);

    // Submit the batch
    const { hash: txHash } = await payyClient.submitTxn({
        txn,
        walletClient,
    });

    const batchHash = await payyClient.hashTxn({ txn, publicClient });

    console.log("Sent stealth batch on-chain tx:", txHash);
    console.log("Batch Txn hash:", batchHash);
}

```
