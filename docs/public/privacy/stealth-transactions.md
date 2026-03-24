# Stealth Transactions

Payy enables privacy on the [EVM Layer](../protocol/evm-layer.md) by making it seamless to move funds in and out of the [Privacy Layer](../protocol/privacy-layer/) (native ERC-20 privacy pools), using a newly minted one-time stealth address for each transaction. For example, to perform a private swap from [PUSD](../stablecoins/pusd.md) to PAYY, private funds are pulled from the Privacy Layer to a new one-time address, the swap is performed on the [EVM Layer](../protocol/evm-layer.md) and PAYY tokens are returned to the Privacy Layer pool after the swap completes.

<figure><img src="../.gitbook/assets/stealth trans.png" alt=""><figcaption></figcaption></figure>

{% hint style="info" %}
Moving funds in and out of the Privacy Layer [requires zero gas](../stablecoins/zero-fee-payments.md), so Stealth Transactions do not incur additional gas over normal EVM transactions.
{% endhint %}

The following diagram describes the flow:

```mermaid
sequenceDiagram
    autonumber
    actor Wallet as Wallet
    participant Vault as Privacy Vault
    participant RPC as RPC
    participant Seq as Sequencer

    Note over Wallet,Vault: Stealth Txn using the Txn standard (single EIP-712 signature, ordered batch calls)

    Wallet->>Vault: Request .stealth() Txn<br/>calls=[approve, swap, sweepBack]<br/>requireSuccess=true, gasLimit, tip
    activate Vault

    Note over Vault: Prepares a one-time stealth address and funds it privately

    Vault->>Vault: Calculate gas budget from gasLimit + tip<br/>Withdraw gas from Privacy Layer pool

    Note over Vault: Authorisation + privacy bridge proofing
    Vault->>Vault: Re-generate Txn signature (EIP-712)<br/>Generate ZK proof for privacy bridge<br/>(private withdraw + gas withdraw)

    Vault->>RPC: Submit single Txn (batched, atomic)<br/>[1] PrivacyBridge.burn(tokenIn → stealth)<br/>[2] PrivacyBridge.burn(gas → stealth)<br/>[3] ERC20.approve(router, amountIn)<br/>[4] DEX.swap(tokenIn→tokenOut, minOut, ...)<br/>[5] PrivacyBridge.mint(tokenOut → vault)<br/>[6] PrivacyBridge.mint(unusedGas → pool)
    RPC->>Seq: Forward Txn to sequencer
    Seq-->>RPC: Include Txn in block & execute calls in order
    RPC-->>Vault: Receipt + per-call results (TxnCallResult)

    Note over Vault,Wallet: If requireSuccess=true, any failure reverts entire sequence<br/>so funds never leave privacy in a partial state

    Vault-->>Wallet: Final result: swap completed privately<br/>tokenOut returned to Privacy Vault<br/>unused gas returned to Privacy Layer pool
    deactivate Vault

```

{% hint style="info" %}
See [Payy Transactions > Stealth](../build-on-payy/payy-transactions/stealth.md) for a guide on using Stealth Transactions.
{% endhint %}

### Gas fees

Moving funds in and out of the Privacy Layer [requires no gas](../stablecoins/zero-fee-payments.md), but stealth transactions themselves require gas for the one-time address that will be responsible for the transaction. The Privacy Vault automatically withdraws gas from the Privacy Layer based on gas limit and gas fee tip, any unused gas is returned to the Privacy Layer when the transaction completes.

