# Schedule

Scheduled payments let users and dapps authorise a payment or batch of actions to execute at a specific time window in the future, without staying online. You sign a Txn (EIP‑712 typed data) that includes a schedule window and submit it to the `TransactionBridge` queue.

This enables recurring payouts, delayed settlements, payroll cycles, subscription retries, and time-based automation while retaining compatibility with existing wallets via typed signatures.

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

async function schedulePayment() {
    const now = Math.floor(Date.now() / 1000);

    // Build scheduled Txn with the schedule prop
    const txn = await payyClient.buildTxn
        .calls([
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xRecipient",
                amount: 1_000_000n,
                gasLimit: 120_000n,
            }),
        ])
        .schedule({
            notBefore: BigInt(now + 60 * 60),         // earliest execution: +1 hour
            notAfter: BigInt(now + 60 * 60 * 24),     // latest execution: +24 hours
        });

    const { hash: queueTxHash } = await payyClient.submitTxn({
        txn,
        walletClient,
    });

    const scheduledHash = await payyClient.hashTxn({ txn, publicClient });

    console.log("Queued on-chain tx:", queueTxHash);
    console.log("Scheduled Txn hash:", scheduledHash);
}
```
