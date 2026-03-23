# Concurrency

Concurrency runs multiple independent pipelines of transactions in parallel using distinct nonce spaces. Instead of a single, globally increasing nonce, you assign a logical key to each workflow (e.g., "payroll", "refunds", "ops"), so transactions in different streams don’t block each other.

Each Txn includes a NonceSpace with:

* key: bytes32 identifier for the stream (e.g., keccak256("payroll"))
* nonce: the expected next nonce for that key

This enables:

* Parallel workflows: submit/queue/execute across different keys without head-of-line blocking.
* Safer batching and retries: replays are scoped to a key; cancelling or failing one stream doesn’t affect others.
* Session keys: ephemeral domains for short-lived automations or per-device nonces.
* Cleaner integration with relayers: deterministic nextNonce(from, key) lookup per stream.

### Best practices:

* Choose human-meaningful keys off-chain and hash to bytes32. Example: `key = keccak256("payroll:2026-Q1")` .
* Increment nonce sequentially per key. Use `nextNonce(from, key)` to fetch the expected value before building a Txn.
* For scheduled or recurring Txns, you can keep the same key or dedicate keys per schedule/recurrence to avoid cross-interference.

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

async function concurrentPayments() {
    // Two independent nonce spaces: "payroll" and "refunds"
    const payrollKey = await payyClient.utils.keccak256Utf8("payroll:2026-Q1");
    const refundsKey = await payyClient.utils.keccak256Utf8("refunds");

    // Look up expected nonces per key
    const payrollNext = await payyClient.nextNonce({
        from: account.address,
        key: payrollKey,
        publicClient,
    });

    const refundsNext = await payyClient.nextNonce({
        from: account.address,
        key: refundsKey,
        publicClient,
    });

    // Build Txn in the payroll nonce space
    const payrollTxn = await payyClient.buildTxn
        .from(account.address)
        .calls([
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xEmployeeA",
                amount: 5_000_000n,
                gasLimit: 120_000n,
            }),
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xEmployeeB",
                amount: 4_500_000n,
                gasLimit: 120_000n,
            }),
        ])
        .nonceSpace({
            key: payrollKey,                    // stream identifier
            nonce: BigInt(payrollNext),         // expected next nonce for this key
        });

    // Build Txn in the refunds nonce space (can proceed in parallel)
    const refundTxn = await payyClient.buildTxn
        .from(account.address)
        .calls([
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xCustomerR",
                amount: 250_000n,
                gasLimit: 80_000n,
            }),
        ])
        .nonceSpace({
            key: refundsKey,
            nonce: BigInt(refundsNext),
        });

    // Send independently; streams won't block each other
    const { hash: payrollTxHash } = await payyClient.submitTxn({
        txn: payrollTxn,
        walletClient,
    });

    const { hash: refundTxHash } = await payyClient.submitTxn({
        txn: refundTxn,
        walletClient,
    });

    const payrollHash = await payyClient.hashTxn({ txn: payrollTxn, publicClient });
    const refundHash = await payyClient.hashTxn({ txn: refundTxn, publicClient });

    console.log("Sent payroll on-chain tx:", payrollTxHash);
    console.log("Sent refund on-chain tx:", refundTxHash);
    console.log("Payroll Txn hash:", payrollHash);
    console.log("Refund Txn hash:", refundHash);
}
```
