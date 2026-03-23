# Node

Primary client for Solid blockchain.

Before running the node, deploy contracts in the `eth` directory and ensure the `chains` section of your configuration includes the deployed rollup contract and RPC endpoint for that chain. Each chain can customize its confirmation depth with the `safe-eth-height-offset` field. The legacy `--eth-rpc-url` and `--rollup-contract-addr` flags still override the first configured chain for convenience.

### Single validators

```bash
cargo run --bin node -- --secret-key="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
```


### Run multiple validators

If you want to test snapshot/restore, you will need at least 4 nodes (with >2/3 majority mode), as with 3 nodes consensus will stall if all 3 are not online.

#### Contract deploy with multiple validators

Before running the nodes, you need to deploy the rollup contract with multiple validators. You can do this by running:

```bash
cd eth
VALIDATORS=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266,0x70997970C51812dc3A010C7d01b50e0d17dc79C8,0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC,0x90F79bf6EB2c4f870365E785982E1f101E93b906 yarn deploy:local
```

#### Node 1

```bash
cargo run --bin node -- --p2p-laddr="/ip4/0.0.0.0/tcp/5001" --p2p-dial="/ip4/127.0.0.1/tcp/5001,/ip4/127.0.0.1/tcp/5002,/ip4/127.0.0.1/tcp/5003,/ip4/127.0.0.1/tcp/5004" --secret-key="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" --rpc-laddr="0.0.0.0:8061" --db-path="~/.polybase/1/db/" --smirk-path="~/.polybase/1/smirk"
```

#### Node 2

```bash
cargo run --bin node -- --p2p-laddr="/ip4/0.0.0.0/tcp/5002" --p2p-dial="/ip4/127.0.0.1/tcp/5001,/ip4/127.0.0.1/tcp/5002,/ip4/127.0.0.1/tcp/5003,/ip4/127.0.0.1/tcp/5004" --secret-key="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" --rpc-laddr="0.0.0.0:8062" --db-path="~/.polybase/2/db/" --smirk-path="~/.polybase/2/smirk"
```

#### Node 3

```bash
cargo run --bin node -- --p2p-laddr="/ip4/0.0.0.0/tcp/5003" --p2p-dial="/ip4/127.0.0.1/tcp/5001,/ip4/127.0.0.1/tcp/5002,/ip4/127.0.0.1/tcp/5003,/ip4/127.0.0.1/tcp/5004" --secret-key="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" --rpc-laddr="0.0.0.0:8063" --db-path="~/.polybase/3/db/" --smirk-path="~/.polybase/3/smirk"
```

#### Node 4

```bash
cargo run --bin node -- --p2p-laddr="/ip4/0.0.0.0/tcp/5004" --p2p-dial="/ip4/127.0.0.1/tcp/5001,/ip4/127.0.0.1/tcp/5002,/ip4/127.0.0.1/tcp/5003,/ip4/127.0.0.1/tcp/5004" --secret-key="0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6" --rpc-laddr="0.0.0.0:8064" --db-path="~/.polybase/4/db/" --smirk-path="~/.polybase/4/smirk"
```

### Tests

To run the E2E tests, you need to deploy contracts for both a single-node setup and a multi-node setup. You can do this with one command:

```bash
cd eth
yarn --silent deploy:tests >../pkg/node/tests/.env.test
```

### Generate new keys

You can generate new public/private key pair by running:

```
cargo run --bin generate_key
```

## RPC

### Get Transaction

`/v0/transaction/${txn_hash}`, where `txn_hash` is the hash of the transaction, in the 0x... format.

### List Transactions

`/v0/transactions`

Query parameters:
- `limit`, max 100, default 10
- `cursor`, which is a string is a cursor from a list response, found in `response.cursor.after` and `response.cursor.before`
- `order`, can be either `"NewestToOldest"` or `"OldestToNewest"`
- `poll`, if true waits for a new transaction if there aren't any to return immediately

### List Blocks

`/v0/blocks`

Query parameters:
- `limit`, max 256, default 10
- `cursor`, same as for transactions
- `order`, either `"LowestToHighest"` or `"HighestToLowest"`
- `skip_empty`, if true, skips blocks with no transactions

### Statistics

#### Transactions

`/v0/stats`

Returns an object containing:
- `last_7_days_txns' - daily transaction count for the last 7 days, excluding today
