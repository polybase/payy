# ERC-8004 Beam App

The ERC-8004 app manages identity-registry agents through Beam's generic app
host. It keeps registry defaults and overrides in app space rather than as a
native Beam command.

```text
beam x erc8004 support
beam x erc8004 config show
beam x erc8004 config set --identity-registry <address>
beam x erc8004 register [--uri <uri>|--empty-uri] [--identity-registry <address>]
beam x erc8004 show <agent-id> [--fetch-uri] [--identity-registry <address>]
beam x erc8004 list [--wallet <wallet>] [--connection owner|agent-wallet|both] [--identity-registry <address>]
beam x erc8004 set-uri <agent-id> <uri> [--identity-registry <address>]
beam x erc8004 set-wallet <agent-id> <wallet> [--deadline-seconds <seconds>] [--identity-registry <address>]
beam x erc8004 unset-wallet <agent-id> [--identity-registry <address>]
```

Default ERC-8004 identity registry addresses are manifest-scoped. Custom
registry addresses come from app-local config or an explicit
`--identity-registry` flag and are included as invocation-scoped contract rules
in host calls and action plans.

`list` uses `eth_getLogs` through the Beam host. The host enforces a bounded
block range and the app defaults to the active wallet with owner filtering, so
it does not scan from genesis unless the user passes a broad explicit range.

`set-wallet` resolves the wallet argument through Beam and requests an EIP-712
typed-data signature from the host. The app receives only the signature and
digest, never raw private keys.
