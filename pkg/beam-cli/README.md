# beam

`beam` is a Rust CLI for day-to-day EVM wallet work. It covers encrypted local wallets,
multi-chain RPC defaults, native asset transfers, ERC20 operations, arbitrary contract
calls, an interactive REPL, and GitHub Releases based self-updates.

The defaults and chain presets are tuned for Payy workflows.

## Install

Install the latest public release:

```bash
curl -L https://install.beam.payy.network | bash
```

Install a specific version:

```bash
curl -L https://install.beam.payy.network | bash -s -- 0.1.0
```

The installer downloads the correct binary for:

- Linux `x86_64`
- macOS `x86_64`
- macOS `aarch64`

Before installing, the script selects the newest stable release that includes the current
platform asset with a valid GitHub Release SHA-256 digest, then verifies the downloaded
binary against that digest and aborts on any mismatch.

Release binaries are built with the bundled Barretenberg bindings backend, so privacy
operations do not require a local `bb` executable. Local development builds keep using the
`bb` CLI backend by default.

Local development install:

```bash
cargo run -p beam-cli -- --help
```

## Quick Start

Create a wallet and make it the default sender:

```bash
beam wallets create
beam wallets list
```

Check tracked balances for your default wallet on Ethereum:

```bash
beam balance
```

`beam balance` always lists the native token first and then every tracked ERC20 for the
selected chain. Use `--from <wallet-name|address|ens>` to change which owner address the
balances are loaded from.

Wallet/address selectors accept a stored wallet name, a raw `0x...` address, or an ENS name
such as `alice.eth`. Beam first checks stored wallet names, then resolves `.eth` inputs
through ENS.

Switch to Base for a single command:

```bash
beam --chain base balance
```

Send native gas token:

```bash
beam --chain sepolia --from alice transfer 0x1111111111111111111111111111111111111111 0.01
```

Estimate gas without signing or submitting a transaction:

```bash
beam --chain sepolia --from alice gas transfer 0x1111111111111111111111111111111111111111 0.01
beam --chain base --from alice gas erc20 transfer USDC 0x1111111111111111111111111111111111111111 1.5
beam --chain base --from alice gas send 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 "transfer(address,uint256)" 0x1111111111111111111111111111111111111111 1000000
```

Check an ERC20 balance:

```bash
beam --chain base balance USDC
beam --chain base balance 0x833589fcd6edb6e08f4c7c32d4f71b54bda02913
```

List and manage tracked tokens:

```bash
beam tokens
beam tokens add 0x833589fcd6edb6e08f4c7c32d4f71b54bda02913
beam tokens add 0x0000000000000000000000000000000000000bee BEAMUSD
beam tokens remove USDC
```

Run an arbitrary contract call:

```bash
beam call 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 "balanceOf(address):(uint256)" 0x1111111111111111111111111111111111111111
```

Inspect a transaction or block:

```bash
beam txn 0xabc123...
beam block latest
```

Inspect deployed contracts:

```bash
beam --chain ethereum contract info 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48
beam --chain ethereum contract bytecode 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 --block latest
beam --chain ethereum contract abi 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48
beam --chain ethereum contract source 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 FiatTokenProxy.sol
beam --chain ethereum contract export 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 ./usdc-source
```

Contract inspection accepts only literal `0x` EVM addresses. `bytecode` reads runtime bytecode from
the active RPC after verifying the RPC chain id; `abi`, `source`, and `export` read runtime-verified
artifacts from Sourcify without explorer API keys. Proxy information is reported as a tip when
Sourcify provides it, but Beam always inspects the exact address you passed. Artifact stdout is
pipeable: `bytecode`, `abi`, and `source <address> <source-path>` print only the requested artifact
in default and compact modes. The bytecode command has no `code` alias. Use `--` before a
`source-path` or `destination` value that begins with `-`.

Start the interactive REPL:

```bash
beam
```

Commands that hit the network show a loading spinner in the default terminal output. In the
REPL, press `Ctrl-C` to cancel an in-flight request and return to the prompt without exiting
the session.

Write commands stop waiting automatically and return a `dropped` state if the active RPC stops
reporting the submitted transaction for roughly 60 seconds.

## Fetch

`beam fetch` is a built-in HTTP client for curl-style requests that can also satisfy x402 and
MPP payment challenges with your active Beam wallet. It makes the initial request directly from
Rust, prints the response body to stdout by default, and retries automatically after a successful
payment when the server answers with `402 Payment Required`.

Supported request flags:

- `-X, --method <METHOD>` to override the HTTP method. Without `-X`, Beam defaults to `GET`,
  or `POST` when `-d`, `--data`, or `--data-file` is present.
- `-H, --header <NAME: VALUE>` to attach repeatable request headers.
- `-d, --data <BODY>` or `--data-file <PATH>` to send a request body.
- `-o, --output <PATH>` to write the response body to a file instead of stdout.
- `-v, --verbose` to print request and response headers on stderr. Beam redacts
  sensitive request header values such as `Authorization`, `Cookie`, and payment
  credentials before printing them.
- `-L, --follow-redirects` with `--max-redirects <N>` to follow redirects on the same
  origin only. Beam stops before a cross-origin hop so origin-scoped headers are not replayed
  to another host.
- `--connect-timeout <SECONDS>` and `--timeout <SECONDS>` for request timing.
- `--no-pay` to print the payment challenge and exit without signing.
- `--max-fee <AMOUNT>` to auto-confirm only when the payment stays within that bound before
  signing. Beam also rejects payments whose estimated gas alone exceeds the cap; native-asset
  payments include the transfer amount plus estimated gas.
- `--allowed-chains <NAME|ID>[,<NAME|ID>...]` to auto-approve only those destination chains for
  payment requests. If a request targets a different chain, Beam fails instead of prompting.
- `--private-payment` to require a privacy-capable challenge recipient and satisfy the payment
  with a private transfer on the selected privacy-capable chain.
- `--dev` to allow plain HTTP payment challenges only for localhost or loopback development
  fixtures. Beam otherwise refuses to pay a `402 Payment Required` response unless the challenged
  URL is `https://`.

Payment flow notes:

- Use `--from <wallet-name|address|ens>` to choose which stored wallet pays for the request.
- Use `--chain <name|id>` to force x402 offer selection. For MPP, it acts as an explicit
  constraint: if the challenge already includes a different `chainId`, Beam fails instead of
  prompting on that network.
- `--chain` and `--allowed-chains` accept the same selectors as other Beam chain commands,
  including canonical names, numeric ids, and aliases like `eth`, `bsc`, `arb`, or `payydev`.
- MPP challenges that omit a chain are rejected unless you explicitly provide `--chain` or
  `--rpc`.
- MPP problem responses must include a valid `WWW-Authenticate: Payment ...` challenge. Beam
  rejects malformed MPP responses on both the paid and `--no-pay` paths.
- When a payment request targets a different chain than your selected/default chain, Beam prompts
  for confirmation unless `--allowed-chains` explicitly permits it.
- Payment challenges served over plain HTTP are rejected unless you opt into `--dev` and the
  challenged URL stays on `localhost` or a loopback address.
- x402 responses are retried with a Beam-generated payment proof header after the payment
  transaction confirms.
- Privacy-capable x402/MPP challenges can include a private recipient address. With
  `--private-payment`, Beam rejects ordinary public challenges and only returns payment
  credentials after the private transfer confirms.
- MPP challenges are retried with an `Authorization: Payment ...` credential after the payment
  transaction confirms. If the original same-origin request already set `Authorization`, Beam
  fails instead of overwriting the caller-supplied credential.
- If a same-origin redirect rewrites the request before the `402 Payment Required` response
  (for example `POST` becoming `GET` on `302`/`303`), Beam retries that effective challenged
  request after payment instead of replaying the pre-redirect method and body.
- In the REPL, once `beam fetch` starts the on-chain payment transaction, `Ctrl-C` stops waiting
  for confirmation without losing the submitted transaction hash. After that transaction phase,
  `Ctrl-C` again cancels the paid retry request or response download and returns to the prompt.

Examples:

```bash
beam fetch https://api.example.com/data
beam fetch -X POST -H "Content-Type: application/json" -d '{"key":"value"}' https://api.example.com/submit
beam fetch --max-fee 0.001 https://paywall.example.com/article/123
beam fetch --allowed-chains base,8453 https://paywall.example.com/article/123
beam fetch --no-pay https://paywall.example.com/article/123
beam --from alice --chain base fetch --max-fee 0.001 https://paywall.example.com/article/123
beam --from alice --chain payy-testnet fetch --private-payment https://paywall.example.com/article/123
beam fetch -v -L https://api.example.com/redirect
```

## Apps

Beam apps are Payy-controlled WASM extensions installed from the Beam registry at
`https://registry.beam.payy.network`. Beam verifies registry signatures and SHA-256
digests before caching app artifacts under `~/.beam/apps`.

Common commands:

```bash
beam apps install erc8004
beam apps install uniswap
beam apps list
beam apps info uniswap
beam apps permissions uniswap
beam apps update uniswap
beam apps remove uniswap
```

Install shows the app publisher, version, registry source, WASM digest, HTTP origins,
chain scopes, contract scopes, function selectors, spender scopes, wallet capabilities,
storage permissions, and privacy capabilities before asking for approval. Use
`--dry-run` to show the same permission summary without activating the app:

```bash
beam apps install uniswap --dry-run --format json
```

Run app commands with the short `x` alias or the explicit lifecycle form:

```bash
beam x uniswap --help
beam x uniswap swap --help
beam --chain base --from alice x uniswap swap USDC ETH 100 --prepare
beam apps run uniswap swap USDC ETH 100 --chain base --from alice --prepare
beam --chain base --from alice x erc8004 support
beam --chain base --from alice x erc8004 register --uri https://agent.example/agent.json
beam --chain base --from alice x erc8004 set-wallet 1 alice
```

Product app business logic lives outside Beam CLI in `beam-apps/apps/<app>`.
Beam CLI owns the generic registry, cache, WASM validation, permission checks,
host ABI, approval records, and execution of approved action plans. Product apps
such as Uniswap and ERC-8004 are built into the registry as WASM and run through
the generic guest command path; Beam CLI does not contain product-specific
built-in planners.

ERC-8004 agent identity management is provided by the `erc8004` app rather than
a native `beam agents` command. Default identity registry addresses are declared
in the app manifest. Custom registry addresses can be persisted with:

```bash
beam x erc8004 config set --identity-registry <address>
```

Per-command registry overrides use `--identity-registry <address>` and are
validated as invocation-scoped contract permissions in the app host.

The Uniswap app will use Beam-mediated HTTPS requests to the Uniswap Trading
API. Release registry builds inject the Payy-managed public Trading API key into
the app artifact from CI:

```bash
export BEAM_UNISWAP_PUBLIC_API_KEY=...
```

The built artifact contains the key, so it is public, rotatable product
configuration rather than a user secret.

For tests and controlled deployments, a registry app manifest can declare a
compatible mocked Trading API endpoint. Beam still enforces the installed app's
declared HTTPS permissions, redirect containment, response limits, chain scopes,
selectors, and spender scopes.

Wallet-affecting app actions are approved by Beam, not by the app. Agents and other
non-interactive callers should prepare a continuation, inspect it, then explicitly
approve and execute it:

```bash
beam --chain base --from alice x <app> <command> --prepare --format json
beam apps approvals show <approval-id>
beam apps approvals approve <approval-id> --execute
```

Beam prices EVM app transactions at approval and execution time. Apps may
propose transaction calldata, value, target, and gas-limit hints, but app
`gas_price`, `maxFeePerGas`, or similar fee fields are informational only and
are not used as the final signed transaction price. On EIP-1559 chains Beam
prefers type-2 transactions; legacy `gas_price` is a fallback for chains that do
not expose EIP-1559 fee history.

Approval prompts and approval JSON include the maximum approved network fee per
transaction step. Pass `--max-network-fee-wei <wei>` to `beam x <app> ...` or
`beam apps run <app> ...` to set a hard per-step network-fee cap; if omitted,
Beam stores a default cap based on the prepared estimate. Execution re-estimates
fees before signing and fails closed if current network fees exceed the approved
cap.

Uniswap token arguments can be configured token labels, `native`, native chain
symbols, or EVM token addresses. Swap options include `--min-receive`,
`--slippage-bps`, `--deadline-seconds`, `--recipient`, `--max-gas`, and
`--unlimited-approval`. Approvals default to the exact amount required and the
swap is only sent after an approval is confirmed or skipped because fresh
allowance is already sufficient. Execution output reports confirmed, pending,
dropped, or skipped transaction state as Beam receives it from the active RPC
path; confirmed receipts include the reported transaction status.

`--no-prompt` fails closed for wallet-affecting app commands unless the command
is preparing a continuation. Removing an app keeps app-local data by default;
pass `--purge-data` to delete `~/.beam/apps/data/<app>` as well.

## Privacy

Beam privacy support is configured per chain. Built-in Payy privacy-capable chains include a
default `payy-evm-privacy` v1 profile. Custom chains become privacy-capable only when `beam chains
add` stores a privacy profile; Beam does not infer privacy support from an RPC endpoint alone.
Built-in Payy chains also include a known ERC20 token label, `native`, for the PUSD predeploy
at `0x0200000000000000000000000000000000000000`, so privacy token arguments can use `native`.

The selected wallet's private address is derived from its encrypted EVM private key with the
SDK-defined `payy/grumpkin/v1` derivation rule. Beam does not store a separate Grumpkin private
key.

Common commands:

```bash
beam --chain payy-testnet privacy address
beam --chain payy-testnet privacy balance [token|token-address]
beam --chain payy-testnet privacy mint native 10
beam --chain payy-testnet privacy incoming list
beam --chain payy-testnet privacy incoming watch
beam --chain payy-testnet privacy mint USDC 10
beam --chain payy-testnet privacy burn USDC 5 0x1111111111111111111111111111111111111111
beam --chain payy-testnet privacy send <private-address> USDC 1
beam --chain payy-testnet privacy send --ephemeral USDC 1 --claim-link-message "invoice"
beam --chain payy-testnet privacy claim <claim-link|incoming-id|artifact>
beam --chain payy-testnet privacy state reset
beam privacy state repair
```

`privacy mint` checks ERC20 allowance before preparing the private proof. If allowance is too low,
run the printed `beam erc20 approve <token> <privacy-bridge> <amount>` command and retry.

Privacy scan state, incoming summaries, owned-note checkpoints, and pending operation records are
stored in `~/.beam/privacy-state.json`. The file is a resume cache, not an authority; Beam
revalidates live bridge state before balance and spend flows. Invalid JSON fails closed. Use
`beam privacy state repair` to move a corrupted state file aside, or `beam privacy state reset` to
clear state for the active wallet and chain.

## Wallets

Wallets are stored in an encrypted local keystore at `~/.beam/wallets.json`.

Supported wallet commands:

```bash
beam wallets create [name]
beam wallets import [--name <name>] [--private-key-stdin | --private-key-fd <fd>]
beam wallets export-private-key [wallet]
beam wallets export-recovery-phrase [wallet]
beam wallets import-recovery-phrase [--name <name>] [--expected-address <address>] [--phrase-stdin | --phrase-fd <fd>]
beam wallets list
beam wallets rename <name|address|ens> <new-name>
beam wallets address [--private-key-stdin | --private-key-fd <fd>]
beam wallets use <name|address|ens>
```

Notes:

- Private keys are encrypted before they are written to disk.
- Each wallet record stores its KDF metadata alongside the encrypted key so future beam releases can keep decrypting older wallets after Argon2 tuning changes.
- `beam wallets import` and `beam wallets address` read the private key from a hidden prompt by default.
- Use `--private-key-stdin` for pipelines and `--private-key-fd <fd>` for redirected file descriptors.
- `beam wallets export-private-key [wallet]` prints the stored wallet's raw primary EVM private key after prompting for the keystore password. When `[wallet]` is omitted, Beam exports the active wallet: the configured default unless `--from` overrides it.
- **Important:** The exported private key gives full control over that wallet. Do not paste it into command arguments, shell variables, issue trackers, chat, screenshots, or logs.
- The exported private key is the primary EVM private key stored by Beam.
- `beam wallets export-recovery-phrase [wallet]` exports a 24-word BIP39 phrase for the selected
  stored wallet. If `[wallet]` is omitted, Beam exports the active wallet: the configured default
  unless `--from` overrides it.
- `beam wallets import-recovery-phrase` imports a wallet from a recovery phrase. By default the
  phrase is read from a hidden prompt; use `--phrase-stdin` for pipelines and `--phrase-fd <fd>`
  for already-open file descriptors.
- Importing a recovery phrase prints the derived EVM wallet address before asking for the new
  wallet password. Use `--expected-address <address>` to fail before persistence if the phrase
  derives a different address than expected.
- Recovery phrases are Payy-compatible entropy backups: Beam maps the 32-byte EVM private key
  directly to and from a 24-word BIP39 phrase. This is not a MetaMask or HD-wallet seed flow; no
  derivation path, account index, or seed expansion is used.
- Importing a recovery phrase restores the same EVM address and the same Payy private address,
  because Beam derives Payy privacy keys from the EVM private key. It does not restore local Beam
  config, scan state, history, custom RPCs, token labels, or pending claim artifacts.
- Treat the phrase exactly like the private key. Avoid storing it in plaintext files; `--phrase-fd`
  is mainly useful for secret-manager streams and tests.
- Do not paste recovery phrases into command arguments or shell variables. Shell history can persist
  those values. Prefer the hidden prompt, `--phrase-stdin` from a secret manager, or `--phrase-fd`
  with an already-open descriptor.
- `beam wallets create` prompts for a wallet name when you omit `[name]`, suggesting the next available `wallet-N` alias and accepting it when you press Enter.
- `beam wallets import` uses a verified ENS reverse record as the default wallet name when one resolves back to the imported address; otherwise it falls back to the next `wallet-N` alias.
- The CLI prompts for a password when creating/importing a wallet. Press Enter at the password prompt to create a wallet with no password; whitespace-only passwords are rejected.
- Beam trims surrounding whitespace and sanitizes terminal control characters in wallet names, rejecting aliases that become empty after normalization.
- Commands that need signing prompt for the keystore password again before decrypting.
- `beam privacy address` uses the same password prompt and keystore integrity checks before
  deriving the wallet's private address.
- If `wallets.json` contains invalid JSON, `beam` fails closed and will not rewrite the file until you repair or restore it.
- Before signing, Beam re-derives the decrypted wallet address and rejects any keystore entry whose key does not match the stored address.
- Wallet names cannot start with `0x`, because that prefix is reserved for raw addresses.
- Wallet names ending in `.eth` must resolve through ENS to that wallet's address before beam accepts them.
- ENS lookups always use the configured Ethereum RPC, and beam rejects that endpoint for ENS if it does not report chain id `1`.
- `--from <name|address|ens>` selects a sender for a single command.
- For signing commands, `--from` must still resolve to a wallet stored in the local keystore, even when you pass a raw address or ENS name.

Examples:

```bash
beam wallets import --name alice
beam wallets rename alice primary
beam --format compact wallets export-private-key alice
beam wallets import --private-key-fd 3 --name alice 3< ~/.config/beam/private-key.txt
beam wallets address --private-key-fd 3 3< ~/.config/beam/private-key.txt
beam wallets export-recovery-phrase alice
beam wallets import-recovery-phrase --name alice
beam wallets import-recovery-phrase --expected-address 0x1111111111111111111111111111111111111111 --name alice
pass show beam/alice/recovery-phrase | beam wallets import-recovery-phrase --phrase-stdin --name alice
beam wallets import-recovery-phrase --phrase-fd 3 --name alice 3< <(pass show beam/alice/recovery-phrase)
```

The signing flow is built on a `Signer` abstraction so hardware-wallet implementations can
be added later without changing command handlers.

## Chains

`beam` ships with built-in presets for:

- Ethereum (`1`)
- Base (`8453`)
- Polygon (`137`)
- BNB (`56`)
- Arbitrum (`42161`)
- Payy Testnet (`7298`)
- Payy Dev (`7297`)
- Sepolia (`11155111`)
- Hardhat (`1337`)

The built-in mainnet and testnet presets default to public RPC endpoints that do not require
an API key. You can still override them per command with `--rpc` or persist a different
default with `beam rpc use`.

Select a chain by name or chain id:

```bash
beam --chain base balance
beam --chain 8453 balance
```

Per-invocation overrides:

- `--chain <name|id>`
- `--rpc <url>`
- `--from <wallet-name|address|ens>`

List chains and RPCs:

```bash
beam chains list
beam rpc list
beam --chain base rpc list
```

Set the default chain:

```bash
beam chains use base
```

Add a custom chain:

```bash
beam chains add "Beam Dev" https://beam.example/dev --chain-id 31337 --native-symbol BEAM
beam chains add "Private Dev" https://beam.example/dev \
  --chain-id 31337 \
  --native-symbol BEAM \
  --privacy-bridge 0x3100000000000000000000000000000000000000 \
  --privacy-features all
```

If you omit the chain name or RPC URL, `beam chains add` prompts for them interactively. When
`--chain-id` is omitted, beam reads the chain id from the RPC endpoint before saving the chain.
When `--chain-id` is provided, beam still verifies that the RPC endpoint reports the same
chain id before persisting the chain. Custom names are trimmed and sanitized for terminal
control characters before they are stored, and they must not reuse an existing selector,
including builtin aliases like `eth`/`bsc` or numeric ids like `1`.

Manage RPCs for the selected chain (either `--chain <name|id>` or the configured default chain):

```bash
beam --chain base rpc add https://beam.example/base-backup
beam --chain base rpc use https://beam.example/base-backup
beam --chain base rpc remove https://beam.example/base-backup
```

Custom chain metadata is stored in `~/.beam/chains.json`. Global defaults and per-chain RPC
configuration live in `~/.beam/config.json`.

Privacy profile fields in `chains.json` include the standard id/version, privacy bridge address,
deployment kind, prover profile, token policy, state policy, and feature flags. `beam chains list`
shows whether each chain has a privacy profile, and JSON output includes the full stored profile.

Beam validates RPC URLs before running a command, so malformed values from `--rpc`,
`config.json`, or `beam chains add` fail with a normal CLI error instead of crashing.

## ERC20 Defaults

`beam` preloads known token metadata into `~/.beam/config.json` on first run and also keeps a
per-chain tracked-token list for `beam balance` and `beam tokens`.

Built-in labels:

- `USDC`
- `USDT`

You can use a label or a raw token address with balance and ERC20 commands:

```bash
beam --chain base balance USDC
beam erc20 transfer 0xTokenAddress 0xRecipient 25
beam erc20 approve USDT 0xSpender 1000
beam tokens add 0xTokenAddress
```

Beam rejects decimal precisions above `77` when converting human-readable values into
on-chain integer units, so hostile token metadata or oversized manual `--decimals`
input fails with a normal CLI validation error instead of crashing.

## Utility Commands

`beam util` exposes the pure/local cast-style helpers that do not require Beam config,
wallets, RPCs, OpenChain, or Etherscan. The command runs as a standalone path, so it works
even when `~/.beam` has not been initialized.

Examples:

```bash
beam util sig "transfer(address,uint256)"
beam util calldata "transfer(address,uint256)" 0x1111111111111111111111111111111111111111 5
beam util abi-encode-event "Transfer(address indexed,address indexed,uint256)" \
  0x1111111111111111111111111111111111111111 \
  0x2222222222222222222222222222222222222222 \
  5
beam util to-wei 1 gwei
beam util from-wei 1000000000 gwei
beam util index address 0x1111111111111111111111111111111111111111 1
beam util create2 --deployer 0x0000000000000000000000000000000000000000 \
  --salt 0x0000000000000000000000000000000000000000000000000000000000000000 \
  --init-code 0x00
```

Supported `beam util` subcommands:

- ABI and calldata: `abi-encode`, `abi-encode-event`, `calldata`, `decode-abi`,
  `decode-calldata`, `decode-error`, `decode-event`, `decode-string`, `pretty-calldata`,
  `sig`, `sig-event`
- Bytes and text: `address-zero`, `concat-hex`, `format-bytes32-string`, `from-bin`,
  `from-utf8`, `hash-zero`, `pad`, `parse-bytes32-address`, `parse-bytes32-string`,
  `to-ascii`, `to-bytes32`, `to-check-sum-address`, `to-hexdata`, `to-utf8`
- Units and number transforms: `format-units`, `from-fixed-point`, `from-wei`, `max-int`,
  `max-uint`, `min-int`, `parse-units`, `shl`, `shr`, `to-base`, `to-dec`,
  `to-fixed-point`, `to-hex`, `to-int256`, `to-uint256`, `to-unit`, `to-wei`
- Hashing, storage, and address derivation: `compute-address`, `create2`, `hash-message`,
  `index`, `index-erc7201`, `keccak`, `namehash`
- RLP: `from-rlp`, `to-rlp`

Several helpers also accept stdin when you omit the positional value, so shell pipelines map
cleanly onto `beam util`.

## Command Reference

Top-level commands:

```bash
beam wallets <subcommand>
beam util <subcommand>
beam chains list
beam chains add [name] [rpc] [--chain-id <id>] [--native-symbol <symbol>] [privacy-flags]
beam chains remove <name|id>
beam chains use <name|id>
beam rpc list [--chain <name|id>]
beam [--chain <name|id>] rpc add [rpc]
beam [--chain <name|id>] rpc remove <rpc>
beam [--chain <name|id>] rpc use <rpc>
beam [--chain <name|id>] tokens [list]
beam [--chain <name|id>] tokens add [token|token-address] [label] [--decimals <decimals>]
beam [--chain <name|id>] tokens remove <token|token-address>
beam [--chain <name|id>] [--from <name|address|ens>] balance [token|token-address]
beam transfer <to> <amount>
beam txn <tx-hash>
beam block [latest|pending|safe|finalized|<number>|<hash>]
beam erc20 balance <token> [name|address|ens]
beam erc20 transfer <token> <to> <amount>
beam erc20 approve <token> <spender> <amount>
beam call <contract> <function-sig> [args...]
beam send [--value <amount>] <contract> <function-sig> [args...]
beam contract info <address>
beam contract bytecode <address> [--block <block>]
beam contract abi <address>
beam contract source <address> [source-path]
beam contract export <address> <destination>
beam privacy address
beam privacy balance [token|token-address]
beam privacy incoming list [--from-block <n>] [--to-block <n>] [--include-spent]
beam privacy incoming watch [--from-block <n>] [--include-spent]
beam privacy mint <token|token-address> <amount>
beam privacy burn <token|token-address> <amount> <recipient>
beam privacy send [--ephemeral] [--memo <bytes32>] [--claim-link-message <text>] [private-address] <token> <amount>
beam privacy claim <claim-link|incoming-id|artifact>
beam privacy state reset
beam privacy state repair
beam fetch [request-flags] <url>
beam update
```

Useful examples:

```bash
beam --format json balance
beam --from alice balance USDC
beam tokens
beam --chain base tokens add 0xTokenAddress
beam chains list
beam --chain base rpc list
beam --chain arbitrum erc20 balance USDT
beam txn 0xTransactionHash
beam block 21000000
beam send 0xContract "approve(address,uint256)" 0xSpender 1000000
beam send --value 0.01 0xContract "deposit()"
beam call 0xContract "symbol():(string)"
beam --chain payy-testnet privacy balance USDC
```

Function signatures use standard ABI signature syntax. For read-only calls, include output
types when you want decoded output, for example:

```bash
beam call 0xContract "name():(string)"
beam call 0xContract "getReserves():(uint112,uint112,uint32)"
```

Write commands wait indefinitely for a mined receipt by default. After Beam has submitted the
transaction, the default terminal loader updates with the transaction hash and pending/mined
status. Press `Ctrl-C` to stop waiting without losing the transaction hash; Beam prints the
submitted hash (and any known block number) so you can keep tracking it with `beam txn` or
`beam block`.

Use `--value` with `beam send` to attach native token to payable contract methods, for
example `beam send --value 0.01 0xContract "deposit()"`.

In the default terminal output mode, RPC-backed commands show a loader while requests are in
flight. Press `Ctrl-C` during a read-only RPC loader to cancel the in-flight request; in the
REPL Beam returns to the prompt, and in one-shot CLI invocations Beam exits with the standard
interrupt status. Successful write commands print the confirmed transaction hash and block so
you can verify the result immediately, while interrupted waits still print the submitted hash.

## Interactive Mode

Running `beam` with no args opens a REPL with history, faded autosuggestions, and tab
completion.

Interactive commands:

```text
wallets <name|address|ens>
chains <name|id>
rpc <url>
balance
tokens
privacy
help
exit
```

Slash-prefixed REPL aliases are not supported. Use bare shortcuts like `wallets <selector>` or
the normal clap command forms such as `wallets create ...` / `beam wallets create ...`.

The REPL also accepts the normal `beam` command set, including flags, nested subcommands,
and clap help output. You can enter those commands either as `transfer ...` / `wallets create`
or with a leading `beam`, and the current wallet, chain, and RPC selections are used as
defaults unless you override them on that command. Interactive startup flags such as
`--chain`, `--from`, and `--rpc` only seed that initial session state. If you later change
the selected wallet, chain, or current chain RPC through a normal CLI subcommand, Beam
reconciles the in-memory REPL selection before rendering the next prompt so renamed or
removed selectors fall back cleanly instead of killing the session. If you later change
chains, Beam falls back to the newly selected chain's configured RPC unless you also choose
another RPC for that chain. The `help` shortcut prints the full CLI help text plus the
REPL-only `exit` command, and both tab completion and inline suggestions follow the same
command tree while also surfacing matching history values. On an empty prompt, `Up` / `Down`
cycle through previously submitted commands. When you type part of a command before pressing
an arrow key, `Up` / `Down` search only history entries with that typed prefix.
The `balance` shortcut prints the full tracked-token report for the current session owner, and
the regular CLI form still handles one-off selectors such as `balance USDC` or `tokens add ...`.
Privacy commands use the regular CLI form under `privacy ...`; tab completion surfaces the privacy
subcommands alongside the rest of the command tree.
When a write command is waiting on-chain, `Ctrl-C` stops the wait, prints the submitted
transaction hash, and returns you to the REPL instead of exiting Beam. Use `Ctrl-D` or `exit`
to leave interactive mode.

The prompt shows the active wallet alias (or raw address override), a shortened address,
the active chain, and the current RPC endpoint.
The chain segment is tinted per network brand in color-capable terminals, and all Payy
networks use `#E0FF32`.

Sensitive wallet and privacy commands are never written to REPL history, and startup immediately
rewrites `~/.beam/history.txt` after scrubbing previously persisted `wallets import` /
`wallets export-private-key` / `wallets import-recovery-phrase` / `wallets address` entries,
including mistyped slash-prefixed variants such as `/wallets import`. `wallets
export-recovery-phrase` may be recorded, but the phrase itself is never part of the command line.
Privacy claim artifacts, ephemeral sends, claim-link messages, memos, and private-payment fetch
commands are also excluded from persisted history.

Interactive startup only reads the cached update status. If a previous background refresh
found a newer GitHub Release, `beam` prints a warning before entering the REPL and refreshes
that cache again in the background when the last GitHub check is older than 24 hours.

If you run `update` from the REPL, beam always relaunches itself after a successful
self-update so you are immediately running the new binary. When the current session still
matches the original startup flags, beam reuses them; otherwise it falls back to a plain
`beam` restart.

## Configuration

Default files:

- `~/.beam/config.json`
- `~/.beam/chains.json`
- `~/.beam/wallets.json`
- `~/.beam/privacy-state.json`
- `~/.beam/history.txt`
- `~/.beam/update-status.json`

To relocate all beam state, set `BEAM_HOME`:

```bash
BEAM_HOME=/tmp/beam beam wallets list
```

`config.json` fields:

- `default_chain`
- `default_wallet`
- `known_tokens`
- `tracked_tokens`
- `rpc_configs` with the configured RPC URLs and default RPC for each chain

`chains.json` stores custom chain metadata added through `beam chains add`.

Selecting a different chain uses that chain's configured RPC unless you also pass `--rpc`
or set `rpc` in the REPL. In interactive mode, changing the session chain clears any prior
session RPC override so the prompt and subsequent commands stay on the selected network.

`beam` also supports structured output modes for scripting:

- `--format default`
- `--format json`
- `--format yaml`
- `--format markdown`
- `--format compact`
- `--format quiet`

Human-facing warnings, errors, and the interactive prompt use color automatically when beam is
writing to a terminal. Override that behavior with `--color auto`, `--color always`, or
`--color never`.

Non-interactive update notices are only printed in `default` output mode and use the cached
update status instead of waiting on GitHub before the command runs.

## Self-Updates

Use:

```bash
beam update
```

The command checks the public `polybase/payy` GitHub Releases feed, selects the newest
stable release that includes a matching binary for the current platform with a valid
GitHub Release SHA-256 digest, downloads that asset, verifies the digest, and only then
swaps the running executable in place.

`beam update` bypasses the normal Beam state bootstrap, so it still reaches the public
GitHub Releases feed even when local `config.json`, `chains.json`, or `wallets.json` need
repair.

Normal startup and non-update commands do not wait on GitHub. They refresh
`update-status.json` asynchronously at most once every 24 hours, and `beam update` remains
the only command that requires the live release check to finish before proceeding.

Release tags use the `beam-v<version>` format and publish assets named:

- `beam-x86_64-unknown-linux-gnu`
- `beam-x86_64-apple-darwin`
- `beam-aarch64-apple-darwin`

The public installer and `beam update` only consider non-draft, non-prerelease
`beam-v<version>` releases from `polybase/payy`, and they only select a release when it
contains the current platform asset with a valid `sha256:` digest. Other repository release
trains do not affect Beam downloads.

The release workflow only publishes a given `beam-v<version>` tag once. If that tag already
exists, reruns skip publication rather than replacing assets, so cut a new Beam version
before triggering another public release.

### Release Control

Beam release versions are prepared in the source `polybase/zk-rollup` repository. The
`.github/workflows/beam.release.bump.yml` workflow opens a release PR for `pkg/beam-cli`
and updates the crate version plus `Cargo.lock`.

Automatic release bumps run after releasable Beam runtime changes land on `main`. The
workflow reads the current public `beam-v<version>` tag in `polybase/payy`, extracts the
mirrored `FolderOrigin-RevId`, and compares that source revision with the current
`zk-rollup` commit. This keeps the same release PR updating on each relevant source push
until it is merged. Conventional commit text from that source range chooses the bump:
`feat` prepares a minor release, breaking changes prepare a major release, and everything
else prepares a patch release. Maintainers can also run the workflow manually with an exact
version or a chosen semver bump.

Before opening another bump PR, the workflow checks that the current Beam version already
has a public `beam-v<version>` tag in `polybase/payy`. This avoids stacking source version
bumps while the previous one is still waiting to be mirrored and published.

The workflow intentionally does not create GitHub Releases. The public `polybase/payy`
action owns the user-facing release. After Copybara mirrors the version change into
`polybase/payy`, `.github/workflows/beam.release.yml` builds the platform binaries and
publishes the public `beam-v<version>` GitHub Release assets that the installer and
`beam update` consume.

## Serving `install.beam.payy.network`

`install.beam.payy.network` should serve `scripts/install-beam.sh` as the public installer entrypoint.

Production serving is owned by the Cloudflare Worker in
`infrastructure/cloudflare/beam-installer`. The Worker embeds
`scripts/install-beam.sh` at deploy time and serves it from `/`, `/install.sh`,
and `/install-beam.sh`.

The deploy workflow is `.github/workflows/beam-installer.release.yml`. It runs on merges
to `main` that touch the installer script, the Worker, or its workflow, and publishes with
Wrangler using the `CLOUDFLARE_API_TOKEN` GitHub secret. The Cloudflare account and zone
ids are configured in the Worker's `wrangler.jsonc`.

After deployment, verify that the public host still serves the canonical script:

```bash
curl -fsSL https://install.beam.payy.network | shasum -a 256
shasum -a 256 scripts/install-beam.sh
```

The release workflow lives in the internal repo but is mirrored into `polybase/payy` via
Copybara so the public repo can publish the assets that `beam update` and the installer
consume.

## Development

From the repository root:

```bash
cargo check -p beam-cli
cargo check -p beam-cli --features payy-evm-client/bb-bindings
cargo test -p beam-cli
```

Full workspace verification is still required before merging:

```bash
cargo xtask lint
cargo xtask test
```
