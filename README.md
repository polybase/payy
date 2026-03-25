<p align="center">
  <img
    src="app/packages/payy/assets/img/payy-logo-wordmark.png"
    alt="Payy logo"
    width="320"
  >
</p>

# Payy - ZK Rollup

An Ethereum L2 zk-rollup for privacy preserving and regulatory compliant transactions.

Here are some highlights:

- Fast - runs in under 3 seconds on an iPhone
- Tiny - UTXO proofs are under 2.8KB
- EVM-compatible - proofs can be verified on Ethereum

For a detailed description of the architecture, download the [whitepaper](https://polybase.github.io/zk-rollup/whitepaper.pdf) or visit the [docs](https://payy.network/docs).


| Module             | Path                                    | Desc                                                            |
|--------------------|-----------------------------------------|-----------------------------------------------------------------|
| Frontends / TypeScript | [app](/app)                         | Frontend applications and TypeScript packages                   |
| Ethereum Contracts | [eth](/eth)                             | Ethereum smart contracts to verify state transitions and proofs |
| Noir               | [noir](/noir)                           | Noir circuits and related tooling                               |
| Aggregator         | [pkg/aggregator](/pkg/aggregator)       | Rollup aggregation services and supporting logic                |
| Node               | [pkg/node](/pkg/node)                   | Core node implementation for the Payy network                   |
| Prover             | [pkg/prover](/pkg/prover)               | Core prover logic                                               |
| RPC                | [pkg/rpc](/pkg/rpc)                     | RPC common utilities shared across all RPC services             |
| Smirk              | [pkg/smirk](/pkg/smirk)                 | Sparse merkle tree                                              |
| ZK-Primitives      | [pkg/zk-primitives](/pkg/zk-primitives) | ZK primitives used across multiple modules                      |


## Git LFS

We use [Git LFS](https://git-lfs.com/) for large files such as proving parameters.

A one-time setup is required for local development:

1. Install `git lfs` by following the instructions at <https://git-lfs.com/>.
2. From the repository root, run:

```bash
git lfs install
git lfs pull
```

## Get Started

There are two core services needed to run the zk rollup stack, and you should start them in order:

1. Eth (with contracts deployed)
2. Node

### Prerequisites

 - [Rust/Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
 - [Go](https://go.dev/doc/install)
 - [Node/nvm/yarn](https://github.com/nvm-sh/nvm?tab=readme-ov-file#installing-and-updating)
 - [Postgres](https://www.postgresql.org/download/)

### Get Started (using VS Code)

You can run all of the services using the VSCode dev container:

Cmd-P -> "Dev Containers: Reopen in Container"


### Get Started (using docker)

You can run all of the services using docker

Run
```bash
docker compose up -f ./docker/docker-compose.yml up -d
```

To only run services that are needed for a dev environment

Run
```bash
docker compose up -f ./docker/docker-compose.yml --profile dev up -d
```


To only run services that are needed for integration tests

Run
```bash
docker compose up -f ./docker/docker-compose.yml --profile test up -d
```


To only run services that are needed for CI workflows

Run
```bash
docker compose up -f ./docker/docker-compose.yml --profile ci up -d
```


To run the prover (optional) to enable withdrawals

Run
```bash
docker compose up -f ./docker/docker-compose.yml --profile prover up -d
```


### Automated Setup

Once the prerequisites above are installed you can bootstrap the local tooling with:

```bash
eval "$(cargo xtask setup)"
```

**What this does:** The `cargo xtask setup` command installs the bb and nargo toolchains, ensures the `polybase-pg` Postgres container is running with the latest migrations, and installs the Ethereum workspace dependencies under `eth/`. It prints shell `export` commands to stdout, and wrapping it in `eval "$(...)"` executes those exports in your current shell so `DATABASE_URL` and any `PATH` updates take effect.

**Environment variables set:**
- `DATABASE_URL` - Connection string for the local Postgres database

**Important:** These exports only persist for the current terminal session. For convenience, consider integrating this command into a repo-specific development shell (for example: direnv, nix shell, guix container) rather than global shell profiles like `.bashrc` or `.zshrc`, because the setup is too heavyweight for global profiles.

Re-run the command whenever you need to refresh the development environment; it is safe and idempotent.

### Targeted Tests

Run the fast test wrapper during development to avoid rebuilding unaffected crates:

```bash
cargo xtask test
```

The command detects workspace crates with local changes (and any dependents), builds tests once via `cargo test --workspace --no-run`, then runs only the compiled test binaries for the affected crates (changed first, then their dependents), exiting early if nothing relevant changed.

### Revi

Download and run the `revi` helper with any arguments (cached under `~/.polybase/revi`):

```bash
cargo xtask revi -- <revi-args>
```

### Local Binaries (debian only)

You will need to install the following packages:

```
apt install libglib2.0-dev libssl-dev libclang-dev python3
```


### Protobuf

Install protobuf

debian:

```
apt install protobuf-compiler libprotobuf-dev
```

macos:

```
brew install protobuf
```

### Fixture Params

Download the proving params before building or running Docker images. This caches the file in
`~/.polybase/fixtures/params` (override with `POLYBASE_PARAMS_DIR`):

```bash
./scripts/download-fixtures-params.sh
```

### Postgres

Install/run postgres and create a db called `guild`.

docker (recommended):

```bash
docker run -it --rm -e POSTGRES_HOST_AUTH_METHOD=trust -e POSTGRES_DB=guild -e POSTGRES_USER=$USER -p 5432:5432 postgres:18
```

macos:

```bash
brew install postgresql
brew services start postgresql
createdb guild
```

debian:

```bash
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
sudo systemctl enable postgresql
sudo -i -u postgres
createdb guild
```

You should be able to connect to the db using:

```bash
psql postgres://localhost/guild
```

(if you're using mac, recommend using [Postico](https://eggerapps.at/postico/v1.php))


### Diesel (for postgres schema setup)

Install diesel CLI:

```bash
cargo install diesel_cli --no-default-features --features postgres
```

Setup the tables in the postgres database:

```bash
$ cd pkg/database
$ diesel migration run
```


### TOML Formatting with Taplo

This repository uses [taplo](https://taplo.tamasfe.dev/) to standardize TOML file formatting across all configuration files, including Cargo.toml, Nargo.toml, and other TOML files.

#### CI Validation

A GitHub Action automatically checks TOML formatting on:
- Pull requests (when TOML files are modified)
- Pushes to `main` branches
- Manual workflow dispatch

The CI will fail if any TOML files don't meet the formatting standards.

#### Installation

Install taplo CLI:

```bash
cargo install taplo-cli --locked
# or
curl -fsSL https://github.com/tamasfe/taplo/releases/latest/download/taplo-<platform>.gz | gzip -d - | install -m 755 /dev/stdin /usr/local/bin/taplo
```

#### Usage

Format all TOML files in the repository:

```bash
taplo fmt
```

Check formatting without making changes:

```bash
taplo fmt --check
```

Validate all TOML files for syntax errors:

```bash
taplo check
```

The formatting configuration is defined in `taplo.toml` at the repository root. The configuration ensures consistent formatting with:
- 2-space indentation
- Multi-line arrays for better readability
- Preserved dependency and key ordering
- Trailing newlines at end of files
- Node modules directories are excluded from checks


### Eth (Ethereum Node)

Setup the [eth node](eth/README.md):

```bash
$ cd eth
$ yarn install
$ yarn eth-node --hostname 0.0.0.0
```

Then deploy the smart contracts to your eth node (in another terminal):

```bash
$ cd eth
$ DEV_USE_NOOP_VERIFIER=1 yarn deploy:local
```

> [!IMPORTANT]
> if you stop the `eth-node` server, you will need to redeploy the contracts again.


### Node (Payy Network)

Run [node](pkg/node/README.md):

```bash
$ cargo run --bin node
```

Run node in prover mode (optional, enables withdrawals):

```bash
$ cargo run --bin node -- --mode mock-prover --db-path ~/.polybase-prover/db --smirk-path ~/.polybase-prover/smirk --rpc-laddr 0.0.0.0:8092 --p2p-laddr /ip4/127.0.0.1/tcp/5001
```

> [!IMPORTANT]
> `eth-node` must be running before starting `node`.

### Guild (API server)
Run [guild](pkg/guild/README.md):

```bash
$ cargo run --bin guild -- --firebase-service-account-path=payy-prenet-firebase.json
```

> [!IMPORTANT]
> `node` must be running before starting `guild`.

### Give yourself some funds

Get the deposit address from the app (Menu -> Deposit -> Deposit Address)

```bash
cargo run --bin wallet transfer <deposit-address> 100
```


## Tests


### Integration tests

```
cargo test integration_test
```

### Rust

```
docker build -f ./docker/Dockerfile.node --target tester .
```

### Workspace hack crate

We use [`cargo-hakari`](https://docs.rs/cargo-hakari) to keep a unified `workspace-hack` crate in sync across all `Cargo.toml` files. Run the following after adding or modifying workspace dependencies and before opening a pull request:

```
cargo hakari generate
cargo hakari manage-deps --yes
```

The `Rust / Hakari Check` GitHub workflow enforces that the crate stays synchronized; if it fails, re-run the commands above and commit the resulting changes.

## Contributing

We welcome contributions that improve the project for everyone.

### Security vulnerabilities

If you discover a security issue, do not report it publicly. Send a full report to [hello@polybaselabs.com](mailto:hello@polybaselabs.com) so it can be handled responsibly.

### Reporting bugs

If you find a bug, open an issue at [github.com/polybase/payy/issues](https://github.com/polybase/payy/issues) with reproduction steps, environment details, and any relevant logs or screenshots.

### Suggesting enhancements

To propose a feature or improvement, open an issue at [github.com/polybase/payy/issues](https://github.com/polybase/payy/issues) and explain the problem, the proposed change, and why it is useful.

### Submitting pull requests

1. Fork the repository.
2. Create a feature branch.
3. Make and test your changes.
4. Commit and push the branch.
5. Open a pull request at [github.com/polybase/payy/pulls](https://github.com/polybase/payy/pulls).
