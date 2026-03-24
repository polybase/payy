# Tokenomics

Payy has two primary native tokens:

1. PAYY - a value accruing token used to lock protocol value
2. PUSD - a USD pegged stablecoin backed by US treasuries, used as the native EVM gas token (can be minted directly from USDC and USDT)

These two tokens are carefully counterbalanced to provide the different properties required for an optimal user experience and value accrual.

### ZKenomics

Privacy is a first party primitive on Payy, and Payy incentivises the use of privacy features to ensure maximum adoption and liquidity in the privacy pools:

* Zero gas on private ERC-20 transfers/payments
* Fixed gas for all other private transactions ($0.01)

Payy is able to offer these features due to the unique properties of ZK (zero knowledge) proofs that are the building blocks of privacy. That is - verifying ZK proofs is extremely fast, non-linear to proof complexity and 100x cheaper than proving.

This enables Payy to implement a high performance ZK payment lane for private payments. Utilising cheap to verify proofs and parallelised execution outside of the EVM, we can ensure that zero gas transfers have limited impact on the network. This allows us to offer a significant user experience boost to consumer use cases, where zero fee transfers are commonplace and expected, driving strong consumer liquidity.

An additional concern associated with offering free transactions is preventing spam. However, the properties of ZK provide natural built-in protection against spam, as proofs are cheaper to verify than they are to create. As clients must generate the proofs and transactions must be submitted in a limited time window, it becomes economically disincentivised to send spam transactions to the network.

### Token Revenue

The PAYY token accrues value in the following ways:

1. gas fees on transactions (avg txn fee target of $0.01)
2. treasury bill yield from PUSD
3. native swaps between PAYY <> PUSD

This revenue model ensures that regardless of whether the user holds PAYY or PUSD, values still accrues to the PAYY token.

#### Gas fees

Whilst direct private payments are free, gas is charged on all other transactions. This includes, all indirect payments which are made as part of another function’s execution. Analysis of recent Ethereum blocks shows that 70% of ERC-20 transfers are made indirectly as part of another call (e.g. swaps, safe). This ensures strong gas revenue for payments.

Gas fees are paid in PUSD, this provides significant UX benefit for payments use cases, as users need to hold only one token for stablecoin payments (regardless of whether they are direct or indirect). Stable gas fees additionally provide a benefit to enterprise use cases, where holding a non-stable token is problematic for economic and legal reasons.

When gas is paid, PUSD is natively swapped to PAYY token and then burned. This actively decreases the supply of the PAYY token for every gas-consuming transaction that occurs on Payy.

Revenue from gas grows with usage:

| **TPS** | **Revenue ($M)** | **Note**            |
| ------- | ---------------- | ------------------- |
| 10      | 3                | -                   |
| 129     | 41               | Tron avg TPS        |
| 173     | 55               | Base avg TPS        |
| 3,000   | 946              | Solana avg TPS      |
| 100,000 | 31,536           | Visa/Mastercard TPS |

_Payy is targeting max throughput of 100,000 tps_

#### Treasury bill yield

Despite most blockchains having a defacto primary stablecoin, most blockchains receive no revenue from their stablecoin tokens. Instead all value accrual flows out of the blockchain to the stablecoin issuer.

On Payy, PUSD will be the primary natively supported stablecoin for Payy network. It is a US dollar denominated yield bearing stablecoin backed by US treasuries. PUSD will not offer yield directly to the user (similar to USDC/USDT), but instead yield will be used to buy and burn the PAYY token.

Because gas fees are paid in PUSD, it’s likely that some portion of user funds will be held in PUSD rather than PAYY token. Yet, because of yield on PUSD, regardless of the allocation between PUSD and PAYY, value still accrues to PAYY token. In addition, because Payy provides gas-free private transfers, we strongly incentivise consumer deposits and liquidity onto Payy, further increasing the yield potential for PUSD.

All yield generated from PUSD is used to buy PAYY token from the native swap pool, and then burned. This actively decreases the supply of the PAYY token for each yield generation event.

Revenue from PUSD yield grows with stablecoin liquidity:

| **TVL**    | **Revenue ($M)** | **Note**                |
| ---------- | ---------------- | ----------------------- |
| **$1B**    | 40               | -                       |
| **$4.41B** | 176              | USDC on Base            |
| **$10B**   | 400              | USDC on Solana          |
| **$78B**   | 3,120            | USDT on Tron            |
| **$156B**  | 6,240            | USDC + USDT on Ethereum |

#### Native swaps

As Payy has two native tokens, it is necessary to provide a native way to swap between them. In particular, as gas fees are paid in PUSD and yield revenue accrues in PUSD, it is necessary to have a native way to swap between the tokens.

A simple AMM model can be used to swap between the two tokens. Swap fees will be charged at 0.1% to ensure strong adoption of the pool. To further enhance PAYY token value accrual, a fee will only be charged when executing a sell on the PAYY token. Buys from USDP to PAYY will not be charged a fee.

The initial liquidity for the native pool will be seeded from the pre-sale and TGE distribution.

Liquidity providers to the native pool will receive PAYY token, all remaining swap fees will be used to burn PAYY token, increasing the price of PAYY for each swap.

### Network participants

Network participants that perform essential roles in the network will be economically incentivised and rewarded with PAYY tokens. These tokens will be newly minted PAYY tokens from the treasury, as is common in other blockchains such as Ethereum.

The following network participants will be rewarded:

* Sequencer - sequence transactions and submission to DA
* Data availability - gas fees paid to Ethereum
* Provers - prove in ZK Payy blocks to be rolled up to Ethereum

Due to the efficiency of Payy’s network design, the cost of rewarding network participants is far less than revenue generated, making PAYY a deflationary value accruing token.
