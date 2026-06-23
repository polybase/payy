# Uniswap App

The Uniswap app turns a swap request into a Beam-approved action plan. It asks
the Uniswap Trading API for a quote, checks your current ERC-20 allowance,
prepares an exact approval only when one is needed, builds the swap transaction,
and hands the whole plan to Beam for approval and execution.

Beam owns your wallet boundary. The app never receives private keys, cannot sign
transactions, and cannot send a transaction on its own.

## Install

```bash
beam apps install uniswap
```

Beam shows the publisher, version, supported chains, network access, wallet
capabilities, and storage permissions before activating the app. To inspect the
same permission summary without installing:

```bash
beam apps install uniswap --dry-run
```

## Swap

```bash
beam x uniswap swap <sell-token> <buy-token> <amount> [options]
```

Command help is exported through the app manifest and rendered by Beam without
fetching a quote or invoking guest WASM:

```bash
beam x uniswap swap --help
```

Example:

```bash
beam x uniswap swap USDC ETH 100 --chain base --from alice
```

Beam shows the quote, any required approval, and the swap as a single plan. You
approve the final plan before Beam signs or submits anything. Beam owns final
transaction pricing: the app may pass a route-specific gas-limit estimate, but
Uniswap API fee values are treated only as route metadata and Beam selects the
signed network fee within the approved cap.

## Options

- `--min-receive <amount>` sets the minimum acceptable output amount.
- `--max-gas <wei>` rejects the plan if estimated gas limit exceeds the limit.
  It is not a max network-fee cap; use Beam's `--max-network-fee-wei <wei>` on
  the app command to cap the per-step network fee.
- `--slippage-bps <bps>` sets max slippage in basis points.
- `--recipient <wallet-or-address>` sends output to another wallet, ENS name, or
  EVM address.
- `--deadline-seconds <seconds>` sets the quote and transaction deadline window.
- `--unlimited-approval` asks Beam to request an unlimited ERC-20 approval
  instead of the default exact approval.

Token inputs can be Beam token labels, `native`, the active chain's native
symbol, or EVM token addresses.

## How a Swap Works

1. The app fetches a quote through Beam-mediated HTTPS access.
2. Beam reads your token balance and allowance through its chain APIs.
3. If allowance is short, Beam adds an exact ERC-20 approval step.
4. The app prepares the swap transaction.
5. Beam renders the typed plan, asks for approval, signs, submits, and tracks the
   receipt.

If an approval is required, Beam submits it first and submits the swap only after
the approval is confirmed. If fresh allowance already satisfies the exact plan,
Beam skips the approval step. Execution output reports confirmed, pending,
dropped, or skipped transaction state; confirmed receipts include the RPC status
value.

## Agents

Agents and other non-interactive callers should prepare a continuation, inspect
it, then explicitly approve and execute it:

```bash
beam x uniswap swap USDC ETH 100 --chain base --from alice --prepare --format json
beam apps approvals show <approval-id>
beam apps approvals approve <approval-id> --execute
```

Prepared approval JSON includes Beam-owned network-fee caps for each executable
step. Old approvals created before fee caps existed require fresh approval
before execution.

`--no-prompt` fails closed for wallet-affecting swaps unless the command is
preparing a continuation or executing an already-approved continuation.

## Permissions

The app requests HTTPS access to the Uniswap Trading API, read/simulate/send
access on supported public EVM chains, ERC-20 approval planning, wallet balance
reads, transaction proposals, and app-local storage. It does not request Beam
privacy capabilities in v1.

Supported chains are Ethereum, Base, Arbitrum, Polygon, BNB, and Sepolia.

Approvals default to the exact amount required. Unlimited approvals require the
explicit `--unlimited-approval` flag and are shown in the Beam approval prompt.
