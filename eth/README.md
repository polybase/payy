# Ethereum Smart Contracts

Rollup smart contracts to verify the rollup state on Ethereum.

## Testing

The project uses Hardhat with Mocha/Chai testing framework and blockchain-specific matchers for comprehensive smart contract testing.

### Run Tests

```bash
yarn test
```

### Test Features

- **Blockchain-specific matchers**: Clean, readable assertions for smart contract testing
- **Balance assertions**: `expect(await contract.balanceOf(owner.address)).to.equal(1000)`
- **Event testing**: `expect(transaction).to.emit(contract, "Transfer")`
- **Revert testing**: `expect(contract.connect(addr1).withdraw()).to.be.revertedWith("Not owner")`
- **Balance change testing**: `expect(tx).to.changeEtherBalance(addr1, ethers.parseEther("1"))`

### Continuous Integration

Tests run automatically on:
- Pull requests modifying files in `eth/`
- Pushes to `main` and `next` branches
- Manual workflow dispatch

All tests must pass with 100% success rate before merge.

## Run locally

Run the local Ethereum hardhat node (resets on each restart):

```bash
yarn eth-node
```

Deploy the contract:

```bash
yarn deploy:local
```

### EIP-7702 delegate deployment

`scripts/deploy_eip7702.ts` now deploys the `Eip7702SimpleAccount` delegate via the
canonical deterministic deployment proxy (`0x4e59…b4956C`) using the salt
`EIP7702_SIMPLE_ACCOUNT_RECEIVE_V1`. The script prints the expected CREATE2 address and
only broadcasts the transaction if the deterministic deployer is available on the selected
network. When running against a development Hardhat chain, the script automatically falls
back to a traditional deployment.

Run server:

```bash
cargo run --release --bin node
```

### Mock aggregate proof

You can deploy a mock aggregate proof verifier using the `DEV_USE_NOOP_VERIFIER=1` environment variable.

You can then run a node with `--mode mock-prover` to skip generating aggregate proofs.

## Deploy to live network

Deploy to a live network. `SECRET_KEY` must have native token on the account. Select network by providing
the network URL

* MAINNET_URL
* SEPOLIA_URL
* MUMBAI_URL
etc

For example:

```bash
SEPOLIA_URL=<alchemy_url> SECRET_KEY=<secret key with eth on network> yarn deploy -- --network sepolia
```

Run server:

```bash
export ETHEREUM_RPC='<same as SEPOLIA_URL>' # maybe I should have just used the same env var names for hardhat deploy
export PROVER_SECRET_KEY=<same as SEPOLIA_SECRET_KEY>
export ROLLUP_CONTRACT_ADDR=...

cargo run --release server
```


### Prenet

#### Deploy

```bash
OWNER=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 PROVER_ADDRESS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 VALIDATORS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 AMOY_URL=<AMOY_URL> SECRET_KEY=<SECRET_KEY> GAS_PRICE_GWEI=2 yarn deploy -- --network amoy
```

#### Upgrade

```bash
ROLLUP_PROXY_ADMIN_ADDR=0x3a7122f0711822e63aa6218f4db3a6e40f97bdcf ROLLUP_CONTRACT_ADDR=0x1e44fa332fc0060164061cfedf4d3a1346a9dc38 AMOY_URL=<AMOY_URL> SECRET_KEY=<SECRET_KEY> yarn upgrade-rollup -- --network amoy
```

Add `UPGRADE_DEPLOY=true` to deploy the contract (not just print the calldata).

### Testnet

#### Deploy

```bash
OWNER=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 PROVER_ADDRESS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 VALIDATORS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 MAINNET_URL=<MAINNET_URL> SECRET_KEY=<SECRET_KEY> yarn deploy -- --network mainnet
```

#### Upgrade

```bash
SECRET_KEY=... ROLLUP_CONTRACT_ADDR=0x883ff65e5ac46fc2c25a4e4ea901bf7e7f6c1705 ROLLUP_PROXY_ADMIN_ADDR=0x90588eedc0c12b8ba50823de7344e1b404384798  MAINNET_URL=<MAINNET_URL> yarn upgrade-rollup -- --network mainnet
```

#### Addresses

```
{
  proverAddress: '0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323',
  validators: [ '0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323' ],
  ownerAddress: '0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323',
  deployerIsProxyAdmin: true
}
Linked library ZKTranscriptLib (__$e6391f3e4b1839f34ea5577896c8005de7$__) at 0x8a15e6434c1036048dfe4832468eddceed98da8a
AGGREGATE_VERIFIER_ADDR=0x69b820ac692cacf2797fd5bd0caff247a2c3cdec
ROLLUP_V1_IMPL_ADDR=0x89b1d6cf21a8c2769e4db4bbb4fdb11097e6a9cf
ROLLUP_CONTRACT_ADDR=0x883ff65e5ac46fc2c25a4e4ea901bf7e7f6c1705
ROLLUP_PROXY_ADMIN_ADDR=0x90588eedc0c12b8ba50823de7344e1b404384798
EIP7702_SIMPLE_ACCOUNT_ADDR=0xd2c66eb938fe5847522891a247264eac90eea93e
```


### Mainnet

```bash
OWNER=0x230Dfb03F078B0d5E705F4624fCC915f3126B40f PROVER_ADDRESS=0x5343B904Bf837Befb2f5A256B0CD5fbF30503D38 VALIDATORS=0x41582701CB3117680687Df80bD5a2ca971bDA964 MAINNET_URL=<MAINNET_URL> SECRET_KEY=<SECRET> yarn deploy -- --network mainnet
```


#### Addresses

```
{
  proverAddress: '0x5343B904Bf837Befb2f5A256B0CD5fbF30503D38',
  validators: [ '0x41582701CB3117680687Df80bD5a2ca971bDA964' ],
  ownerAddress: '0x230Dfb03F078B0d5E705F4624fCC915f3126B40f',
  deployerIsProxyAdmin: false
}
Linked library ZKTranscriptLib (__$e6391f3e4b1839f34ea5577896c8005de7$__) at 0xab57f01dc6cffd313233ec14d474cdf82512ff66
AGGREGATE_VERIFIER_ADDR=0xb553c325959c8615d9018f00906aec3799b94200
ROLLUP_V1_IMPL_ADDR=0x7d8837b547f4fea0053571cb149e845fc58e9b2d
ROLLUP_CONTRACT_ADDR=0x367c1eaf14aa06b78ce76bd0243297de79d85270
ROLLUP_PROXY_ADMIN_ADDR=0xfe455bacaf1968f1ae6a322b8ffbe56840e2f590
EIP7702_SIMPLE_ACCOUNT_ADDR=0x63b925fe7096104471bcbee5358505bf6e892344
```

#### Upgrade

```bash
SECRET_KEY=... ROLLUP_CONTRACT_ADDR=0x367c1eaf14aa06b78ce76bd0243297de79d85270 ROLLUP_PROXY_ADMIN_ADDR=0xfe455bacaf1968f1ae6a322b8ffbe56840e2f590  MAINNET_URL=<MAINNET_URL> yarn upgrade-rollup -- --network mainnet
```

### Upgrade Rollup contract

Fresh deployments now use the merged `contracts/rollup3/RollupV1.sol` implementation, so the
upgrade flow is only required for legacy rollup2 deployments.

Using `yarn upgrade-rollup`, you can upgrade a previously deployed rollup contract to a new version.

Example without a specified network:

```bash
SECRET_KEY=... ROLLUP_CONTRACT_ADDR=<proxy_contract_addr> ROLLUP_PROXY_ADMIN_ADDR=<proxy_admin_contract_addr> yarn upgrade-rollup
```

## Mint migration (Polygon -> Ethereum)

Migration is a three-step process:

1. **Extract**: `scripts/extract-mints.ts` reads `MintAdded` events from the source chain and saves unspent mints to a JSON file.
2. **Filter**: `scripts/filter-mints.ts` verifies candidates against the source chain state (via `getMint`) to ensure they are valid and unspent.
3. **Submit**: `scripts/submit-mints.ts` reads the verified JSON file, filters out mints already present on the target chain, and submits the rest.

### Step 1: Extract Mints

Required environment variables:
- `SOURCE_RPC_URL`: Source chain RPC endpoint
- `SOURCE_CONTRACT_ADDRESS`: Source rollup contract address

Optional:
- `START_BLOCK`, `END_BLOCK`: Scan range
- `OUTPUT_FILE`: Path to save mints (default: `mints.json`)
- `BLOCK_BATCH_SIZE`: Blocks per scan chunk (default: 10000)
- `RPC_CONCURRENCY`: Concurrent requests (default: 5)

```bash
SOURCE_RPC_URL=<polygon_url> SOURCE_CONTRACT_ADDRESS=<addr> yarn extract-mints
```

### Step 2: Filter Mints

Required environment variables:
- `SOURCE_RPC_URL`: Source chain RPC endpoint
- `SOURCE_CONTRACT_ADDRESS`: Source rollup contract address

Optional:
- `INPUT_FILE`: Path to read extracted mints (default: `mints.json`)
- `OUTPUT_FILE`: Path to save verified mints (default: `filtered-mints.json`)
- `RPC_CONCURRENCY`: Concurrent verification requests (default: 10)

```bash
SOURCE_RPC_URL=<polygon_url> SOURCE_CONTRACT_ADDRESS=<addr> yarn filter-mints
```

### Step 3: Submit Mints

Required environment variables:
- `TARGET_RPC_URL`: Target chain RPC endpoint
- `TARGET_CONTRACT_ADDRESS`: Target rollup contract address
- `PRIVATE_KEY`: Private key for transaction submission

Optional:
- `INPUT_FILE`: Path to read verified mints (default: `filtered-mints.json`)
- `DRY_RUN`: Set to `true` to skip transactions
- `PRINT_TX`: Set to `true` to print transaction data (for Safe execution) instead of sending
- Gas fees are set automatically to 2x the RPC-recommended EIP-1559 fees (`maxFeePerGas` and `maxPriorityFeePerGas`)

```bash
TARGET_RPC_URL=<eth_url> TARGET_CONTRACT_ADDRESS=<addr> PRIVATE_KEY=<key> INPUT_FILE=filtered-mints.json yarn submit-mints
```

## Rollup initializer requirements

`RollupV1.initialize` now accepts eight arguments:

```solidity
function initialize(
    address owner,
    address usdcAddress,
    address verifierAddress,
    address prover,
    address[] calldata initialValidators,
    bytes32 verifierKeyHash,
    uint32 verifierMessagesCount,
    bytes32 initialNoteKind
);
```

- `verifierMessagesCount` must match the `messages_length` that your aggregate verifier enforces. The legacy `agg_final` circuit expects 1,000 messages (6 UTXOs * 5 messages per UTXO * padding), but if you regenerate a verifier with a different circuit you **must** update this value so the contract can size-check proofs before calling into the verifier. You can source the count from:
  - the `messages_length` field on the deployed verifier contract (visible via `rollup.zkVerifiers(keyHash).messages_length`), or
  - the generator metadata emitted by `noir/generate_fixtures.sh`. Keep the value alongside the verifier address in your deployment runbook.
- `initialNoteKind` binds the rollup to the token it will mint/burn. For bridged EVM assets we use format `0x0002 || chain_id(uint64, big endian) || token_address(20 bytes)`. The helper in [`eth/scripts/noteKind.ts`](./scripts/noteKind.ts) (`generateNoteKindBridgeEvm(chainId, tokenAddress)`) produces the correct 32-byte value; reuse it (e.g. through `ts-node`) when onboarding new tokens so deposits, withdrawals, and burn substitutions resolve to the right ERC20 contract.

Record both arguments in your deployment checklist—operators must replay them any time they redeploy or upgrade through `TransparentUpgradeableProxy`.

## Security Improvements

### Block Height Validation (ENG-4064)

The `verifyRollup` function in `contracts/rollup3/RollupV1.sol` now includes validation to ensure new block heights are strictly greater than the current block height. This prevents:

- **Rollback Attacks**: Malicious actors cannot submit blocks with decreasing heights
- **Replay Attacks**: Same block height cannot be reused
- **Sequencing Integrity**: Maintains proper rollup block ordering
- **State Inconsistency**: Prevents breaking dependent systems expecting monotonic height increases

The validation is implemented as:
```solidity
require(height > blockHeight, "RollupV1: New block height must be greater than current");
```

### Testing

Run the security tests with:
```bash
yarn test test/SimpleBlockHeightTest.test.ts
```

## Verifier Addresses

This table documents deployed verifier contract addresses and their configurations:

| Verifier Address | Verification Key | Messages | Notes |
|------------------|------------------|----------|-------|
| `0xe859860f654da247ba1468785ea40a386e982110` | `0x122d2ac7542fa020cbfff0836b5d0c30898330074b19869179bba49b5db69967` | 1000 | AGG_FINAL verifier |

**Verification Key** corresponds to `AGG_FINAL_VERIFICATION_KEY_HASH`, which is consumed by the rollup verifier logic in [pkg/contracts/src/rollup.rs](../pkg/contracts/src/rollup.rs) and deployment tooling.

**Messages** reflects the `verifierMessagesCount` value you supplied during `initialize`. For the current `agg_final` deployment this still equals the historical `AGG_FINAL_MESSAGES` constant (1000), but if you deploy a verifier with a different circuit you must update both the table and the initializer arguments so every operator knows which count to pass.

Verifier contracts and their verification keys are generated via [noir/generate_fixtures.sh](../noir/generate_fixtures.sh); rerun that script whenever a circuit changes. Update this table whenever a new verifier is deployed so downstream operators have a single source of truth.

## Regenerating EVM aggregate proof verifier

To re-generate EVM proof verifier, see [pkg/contracts](/pkg/prover).
