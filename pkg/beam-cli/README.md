# beam

`beam` is a Rust CLI for day-to-day EVM wallet work. It covers encrypted local wallets,
multi-chain RPC defaults, native asset transfers, ERC20 operations, arbitrary contract
calls, an interactive REPL, and GitHub Releases based self-updates.

The defaults and chain presets are tuned for Payy workflows.

## Install

Install the latest public release:

```bash
curl -L https://beam.payy.network | bash
```

Install a specific version:

```bash
curl -L https://beam.payy.network | bash -s -- 0.1.0
```

The installer downloads the correct binary for:

- Linux `x86_64`
- macOS `x86_64`
- macOS `aarch64`

Before installing, the script selects the newest stable release that includes the current
platform asset with a valid GitHub Release SHA-256 digest, then verifies the downloaded
binary against that digest and aborts on any mismatch.

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

Start the interactive REPL:

```bash
beam
```

Commands that hit the network show a loading spinner in the default terminal output. In the
REPL, press `Ctrl-C` to cancel an in-flight request and return to the prompt without exiting
the session.

Write commands stop waiting automatically and return a `dropped` state if the active RPC stops
reporting the submitted transaction for roughly 60 seconds.

## Wallets

Wallets are stored in an encrypted local keystore at `~/.beam/wallets.json`.

Supported wallet commands:

```bash
beam wallets create [name]
beam wallets import [--name <name>] [--private-key-stdin | --private-key-fd <fd>]
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
- `beam wallets create` prompts for a wallet name when you omit `[name]`, suggesting the next available `wallet-N` alias and accepting it when you press Enter.
- `beam wallets import` uses a verified ENS reverse record as the default wallet name when one resolves back to the imported address; otherwise it falls back to the next `wallet-N` alias.
- The CLI prompts for a password when creating/importing a wallet and rejects empty or whitespace-only values.
- Beam trims surrounding whitespace and sanitizes terminal control characters in wallet names, rejecting aliases that become empty after normalization.
- Commands that need signing prompt for the keystore password again before decrypting.
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
printf '%s\n' "$BEAM_PRIVATE_KEY" | beam wallets import --private-key-stdin --name alice
beam wallets address --private-key-fd 3 3< ~/.config/beam/private-key.txt
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
beam chains add [name] [rpc] [--chain-id <id>] [--native-symbol <symbol>]
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
beam update
```

Useful examples:

```bash
beam --output json balance
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
command tree while also surfacing matching history values. When you have typed part of a
command, `Up` / `Down` search only history entries with that prefix; on an empty prompt they
cycle through previously submitted commands.
The `balance` shortcut prints the full tracked-token report for the current session owner, and
the regular CLI form still handles one-off selectors such as `balance USDC` or `tokens add ...`.
When a write command is waiting on-chain, `Ctrl-C` stops the wait, prints the submitted
transaction hash, and returns you to the REPL instead of exiting Beam. Use `Ctrl-D` or `exit`
to leave interactive mode.

The prompt shows the active wallet alias (or raw address override), a shortened address,
the active chain, and the current RPC endpoint.
The chain segment is tinted per network brand in color-capable terminals, and all Payy
networks use `#E0FF32`.

Sensitive wallet commands are never written to REPL history, and startup immediately rewrites
`~/.beam/history.txt` after scrubbing previously persisted `wallets import` / `wallets address`
entries, including mistyped slash-prefixed variants such as `/wallets import`.

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

- `--output default`
- `--output json`
- `--output yaml`
- `--output markdown`
- `--output compact`
- `--output quiet`

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

## Serving `beam.payy.network`

`beam.payy.network` should serve `scripts/install-beam.sh` as the public installer entrypoint.

One straightforward setup is:

1. Publish `scripts/install-beam.sh` to a static host such as GitHub Pages.
2. Configure the host to serve the script at `/`.
3. Point the `beam.payy.network` DNS record at that static host.
4. Keep the script in sync with the current public GitHub Releases asset naming scheme.

The release workflow lives in the internal repo but is mirrored into `polybase/payy` via
Copybara so the public repo can publish the assets that `beam update` and the installer
consume.

If you use GitHub Pages, a simple `CNAME` record from `beam.payy.network` to the Pages host
is enough as long as the root URL responds with the installer script body.

## Development

From the repository root:

```bash
cargo check -p beam-cli
cargo test -p beam-cli
```

Full workspace verification is still required before merging:

```bash
cargo xtask lint
cargo xtask test
```
