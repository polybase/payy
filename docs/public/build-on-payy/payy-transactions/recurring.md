# Recurring

Recurring schedules let users and dapps authorise a payment or batch of actions to execute repeatedly on a defined cadence (e.g., weekly, monthly), without staying online. You create a Recurrence (EIP‑712 typed data) that references a base Txn template and a cadence. The bridge materialises each occurrence into a concrete Txn with a derived schedule window and unique salt, then queues or submits it when due.

This enables payroll cycles, streaming-like payouts, subscription charges with retries, periodic settlements, and routine maintenance tasks while retaining compatibility with existing wallets via typed signatures.

### Key concepts:

* Recurrence: a signed template containing the base Txn, cadence, bounds, and optional queue-ahead instructions.
* Occurrence: a concrete Txn derived from the template for a specific time window and index, with deterministic salt for replay safety.
* Keepers: anyone can call queueNextOccurrence or submitNextOccurrence when due. If queueAhead is enabled, fillQueueAhead can pre-queue several future occurrences.

