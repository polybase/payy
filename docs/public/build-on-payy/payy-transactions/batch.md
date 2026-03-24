# Batch

Batch lets you make multiple calls to execute atomically in one Txn using the [TransactionBridge](../../protocol/transactionbridge.md). You sign a single Txn (EIP‑712 typed data) that includes an ordered array of calls. The bridge executes them in sequence within one transaction.

You can choose:

* `requireSuccess: true` - all calls must succeed or the whole batch reverts (i.e. atomic).
* `requireSuccess: false` - best‑effort, failing calls don’t revert the entire batch, and per‑call results are emitted in `TxnCallResult`. The transaction still finishes in the processed lifecycle state, while `TxnProcessed.success` flips to `false` if any subcall fails.

This enables multi‑step workflows like approve + swap + transfer, complex settlements, multi‑recipient payouts, or protocol operations that must move together.

### Example

```typescript
import { createPublicClient, createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import {
    erc20,
    createPayyClient,
} from "@payy/viem";
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

async function batchPayment() {
    // Build a batch with three transfers, atomic by default (requireSuccess: true)
    const txn = await payyClient.buildTxn
        .calls([
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xAlice",
                amount: 1_000_000n,
                gasLimit: 80_000n,
            }),
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xBob",
                amount: 2_500_000n,
                gasLimit: 80_000n,
            }),
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xCarol",
                amount: 750_000n,
                gasLimit: 80_000n,
            }),
        ])
        .requireSuccess(true); // ensure all transfers succeed or all revert

    // Optional: best‑effort mode (partial success, emits per‑call results)
    // .requireSuccess(false);

    // Send the batch on-chain
    const { hash: txHash } = await payyClient.submitTxn({
        txn,
        walletClient,
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });

    const batchHash = await payyClient.hashTxn({ txn, publicClient });

    console.log("Sent batch on-chain tx:", txHash);
    // For best-effort batches, inspect receipt logs for TxnCallResult / TxnProcessed.
    console.log("Batch receipt status:", receipt.status);
    console.log("Batch Txn hash:", batchHash);
}
```
