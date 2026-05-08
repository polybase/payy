# Batch

{% hint style="warning" %}
TransactionBridge batch builders are not implemented in the current Payy client SDK releases. `client.transactions()` is reserved for this future surface.
{% endhint %}

Batch transactions execute multiple calls through the [`TransactionBridge`](../../protocol/transactionbridge.md) in one ordered transaction.

Planned shape:

```typescript
const batch = await client
  .transactions()
  .batch({
    from: evmAccount,
    requireSuccess: true,
    calls: [
      { to: token, data: transferAliceCalldata, value: 0n, gasLimit: 80_000n },
      { to: token, data: transferBobCalldata, value: 0n, gasLimit: 80_000n },
    ],
  })
  .prepare();

await batch.submitAndWait();
```

Until this SDK surface exists, construct and submit TransactionBridge calldata directly if you need contract-level batch behavior.
