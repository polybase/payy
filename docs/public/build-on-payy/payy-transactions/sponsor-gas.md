# Sponsor Gas

{% hint style="warning" %}
TransactionBridge sponsored-gas builders are not implemented in the current Payy client SDK releases. `client.transactions()` is reserved for this future surface.
{% endhint %}

Sponsored gas lets a sponsor or paymaster authorize fee payment for a TransactionBridge request.

Planned shape:

```typescript
const sponsored = await client
  .transactions()
  .batch({
    from: evmAccount,
    calls,
    feePayer: {
      mode: "sponsored",
      payer: sponsor,
      authData: sponsorAuthorization,
    },
  })
  .prepare();

await sponsored.submitAndWait();
```

Until this SDK surface exists, use the TransactionBridge contract fee-payer fields directly.
