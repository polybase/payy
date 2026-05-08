# Recurring

{% hint style="warning" %}
TransactionBridge recurring-payment builders are not implemented in the current Payy client SDK releases. `client.transactions()` is reserved for this future surface.
{% endhint %}

Recurring transactions are planned templates that derive scheduled TransactionBridge requests over time. They are intended for subscriptions, payroll cycles, periodic settlement, and maintenance tasks.

Planned shape:

```typescript
const recurring = await client
  .transactions()
  .recurring({
    from: evmAccount,
    calls,
    cadence: "monthly",
    startsAt,
    endsAt,
  })
  .prepare();

await recurring.submitAndWait();
```

Until this SDK surface exists, recurring execution must be coordinated outside the SDK by preparing concrete TransactionBridge requests for each occurrence.
