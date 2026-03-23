# Private Transfers

You don't need to make any modifications to make private native token or ERC-20 transfers. For details on how private transfers are enabled without any modifications, see [Private Transfers](../privacy/editor.md) overview.

{% hint style="info" %}
All private token transfers are gas zero rated to enable [zero fee private payments](../stablecoins/zero-fee-payments.md).
{% endhint %}

### Private native token transfer

You'll notice that there are no changes required to send private native transfers, yet if you have tokens in your [Privacy Vault](../protocol/privacy-vault.md), they will be automatically pulled from the [Privacy Layer](../protocol/privacy-layer/) and used in the transfer.

```typescript
import { createPublicClient, createWalletClient, http, parseEther } from "viem";
import { payy } from "@payy/viem/chains";
import { privateKeyToAccount } from "viem/accounts";

const account = privateKeyToAccount(process.env.PRIVATE_KEY as `0x${string}`);

const publicClient = createPublicClient({
    chain: payy,
    transport: http(process.env.RPC_URL),
});

const walletClient = createWalletClient({
    account,
    chain: payy,
    transport: http(process.env.RPC_URL),
});

async function sendNative() {
    const to = "0xRecipientAddressHere" as `0x${string}`;
    const hash = await walletClient.sendTransaction({
        to,
        value: parseEther("0.01"),
    });

    console.log("tx hash:", hash);
    const receipt = await publicClient.waitForTransactionReceipt({ hash });
    console.log("status:", receipt.status);
}

sendNative().catch(console.error);
```

