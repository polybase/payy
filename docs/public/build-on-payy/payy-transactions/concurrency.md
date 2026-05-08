# Concurrency

{% hint style="warning" %}
TransactionBridge concurrency builders are not implemented in the current Payy client SDK releases. `client.transactions()` is reserved for this future surface.
{% endhint %}

Concurrency uses distinct nonce spaces so independent workflows can progress without blocking one another. Each TransactionBridge request carries a nonce-space key and nonce.

Planned shape:

```typescript
const payroll = await client
  .transactions()
  .batch({
    from: evmAccount,
    nonceSpace: { key: payrollKey, nonce: payrollNonce },
    calls: payrollCalls,
  })
  .prepare();

const refunds = await client
  .transactions()
  .batch({
    from: evmAccount,
    nonceSpace: { key: refundsKey, nonce: refundsNonce },
    calls: refundCalls,
  })
  .prepare();

await Promise.all([payroll.submitAndWait(), refunds.submitAndWait()]);
```

Until this SDK surface exists, use the TransactionBridge contract nonce-space fields directly.
