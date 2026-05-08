# Stealth

{% hint style="warning" %}
TransactionBridge stealth builders are not implemented in the current Payy client SDK releases. `client.transactions()` is reserved for this future surface.
{% endhint %}

Stealth execution is the planned bridge between private balances and temporary public EVM execution. The intended flow is:

1. Spend private value into a temporary EVM account.
2. Execute one or more public EVM calls from that temporary account.
3. Return leftovers to the privacy layer with a proof-backed `mint(...)`.

Planned shape:

```typescript
const stealth = await client
  .transactions()
  .stealth({
    privacyAccount,
    token,
    amount,
    calls: [
      { to: router, data: swapCalldata, value: 0n, gasLimit: 250_000n },
    ],
  })
  .prepare();

await stealth.submitAndWait();
```

Today, the implemented SDK pieces are the privacy operations needed around this design: `send().to(...)`, `send().ephemeral(...)`, `claim()`, `mint(...)`, and `burn(...)`. The high-level stealth transaction builder is not yet available.
