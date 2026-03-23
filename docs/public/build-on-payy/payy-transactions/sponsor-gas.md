# Sponsor Gas

Sponsor Gas lets a third party (a sponsor/paymaster) cover the execution costs of a Txn so the sender doesn’t need to hold gas tokens. You sign a Txn (EIP‑712 typed data) with fee.model set to Sponsored and include sponsor information. The bridge validates the sponsor’s policy and settles fees accordingly.

This enables:

* Gasless onboarding and first‑use flows
* Subscriptions and dapps covering user gas
* Enterprise workflows with centralised gas budgets
* Promotional campaigns and fee rebates

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

async function gasSponsoredTransfer() {
    // Build a Txn with fee.model = Sponsored (1)
    const txn = await payyClient.buildTxn
        .calls([
            erc20.transfer({
                token: "0xYourERC20",
                to: "0xRecipient",
                amount: 1_000_000n,
                gasLimit: 120_000n,
            }),
        ]);
        
    // Fetch sponsor sig for the designated fee
    const sponsorSig = await fetchSponsorSig(txn); // returns a signature approving fee sponsorship

    // Add the fee signature to the txn
    const txnWithFeeSponsor = txn.withSponsor(sponsorSig);

    // Send the sponsored Txn on-chain (no native value needed from the user)
    const { hash: txHash } = await payyClient.submitTxn({
        txn: txnWithFeeSponsor,
        walletClient,
    });

    const txnHash = await payyClient.hashTxn({ txn: txnWithFeeSponsor, publicClient });

    console.log("Sent sponsored on-chain tx:", txHash);
    console.log("Txn hash:", txnHash);
}
```
