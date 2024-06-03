# ZK Circuits


## Regenerating EVM verifiers

If the aggregate_verifier is modified (i.e. as a result of the aggregation proof being modified), then we need to EVM verifiers.

See [Scroll's ZkEvmVerifierV1.sol](https://github.com/scroll-tech/scroll/blob/4aa5d5cd37649b26d442147e9c2b79e330ba1a2f/contracts/src/libraries/verifier/ZkEvmVerifierV1.sol#L37) code for how to call this verifier from Solidity.


### Aggregate

```sh
UPDATE_EXPECT=1 cargo test "aggregate_agg::tests::generate_verifier" --release
```

Verifier YUL code will be generated in `pkg/zk-circuits/src/aggregate_agg/aggregate_verifier.yul`.

```sh
solc --bin --yul pkg/zk-circuits/src/aggregate_agg/aggregate_verifier.yul | grep -E '^[0-9a-fA-F]+$' >eth/contracts/AggregateVerifier.bin
```

### Mint

```sh
UPDATE_EXPECT=1 cargo test "mint::tests::generate_verifier" --release
```

Verifier YUL code will be generated in `pkg/zk-circuits/src/mint/mint_verifier.yul`.

```sh
solc --bin --yul pkg/zk-circuits/src/mint/mint_verifier.yul | grep -E '^[0-9a-fA-F]+$' >eth/contracts/MintVerifier.bin
```

### Burn

```sh
UPDATE_EXPECT=1 cargo test "mint::burn::generate_verifier" --release
```

Verifier YUL code will be generated in `pkg/zk-circuits/src/burn/burn_verifier.yul`.

```sh
solc --bin --yul pkg/zk-circuits/src/burn/burn_verifier.yul | grep -E '^[0-9a-fA-F]+$' >eth/contracts/BurnVerifier.bin
```

