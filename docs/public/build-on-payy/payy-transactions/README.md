# Payy Transactions

{% hint style="warning" %}
High-level TransactionBridge builders are not implemented in the current Payy client SDK releases. The SDKs reserve `client.transactions()` for this surface, but these pages describe the intended contract-level workflows, not callable SDK methods.
{% endhint %}

Payy Transactions are planned high-level builders for the [`TransactionBridge`](../../protocol/transactionbridge.md): batching, nonce-space concurrency, sponsored gas, schedules, recurrence, and stealth execution.

Current SDK support is focused on the privacy layer through the [Payy client SDKs](../payy-client/README.md). Use these pages as design guidance until the TransactionBridge SDK surface is implemented.

```typescript
const transactions = client.transactions();

// Reserved namespace. Concrete builders are not available yet.
```
