# Schedule

{% hint style="warning" %}
TransactionBridge schedule builders are not implemented in the current Payy client SDK releases. `client.transactions()` is reserved for this future surface.
{% endhint %}

Scheduled transactions authorize execution within a future time window. The TransactionBridge validates the schedule fields before processing the call set.

Planned shape:

```typescript
const scheduled = await client
  .transactions()
  .batch({
    from: evmAccount,
    calls,
    schedule: {
      notBefore: BigInt(Math.floor(Date.now() / 1000) + 3600),
      notAfter: BigInt(Math.floor(Date.now() / 1000) + 86_400),
    },
  })
  .prepare();

await scheduled.submitAndWait();
```

Until this SDK surface exists, use the TransactionBridge contract schedule fields directly.
