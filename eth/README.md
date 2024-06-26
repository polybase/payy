# Ethereum Smart Contracts

Rollup smart contracts to verify the rollup state on Ethereum.

## Run locally

Run the local Ethereum hardhat node (resets on each restart):

```bash
yarn eth-node
```

Deploy the contract:

```bash
yarn deploy:local
```

Run server:

```bash
ROLLUP_CONTRACT_ADDR=<from deploy step> cargo run --release server
```


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

```bash
OWNER=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 PROVER_ADDRESS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 VALIDATORS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 AMOY_URL=<ETH_RPC> SECRET_KEY=<SECRET_KEY> GAS_PRICE_GWEI=2 yarn deploy -- --network amoy
```

Addresses:

```
USDC_CONTRACT_ADDR=0x206fcb3bea972c5cd6b044160b8b0691fb4aff57
AGGREGATE_BIN_ADDR=0x58f2e5031af2d6c1996334b10880973c494e3b06
AGGREGATE_VERIFIER_ADDR=0xa98e2c3a375b5aedf31b1276594a11ff41d72a36
MINT_BIN_ADDR=0x3945f7f99460c86dfe73de6a757b1b6ed1a52604
MINT_VERIFIER_ADDR=0xfeda1cec4b2b9f958e6c0823cf14b0e687fa4a59
BURN_BIN_ADDR=0xaa331ab85fa49137cbfbb614bc20eb55e0e1ae46
BURN_VERIFIER_ADDR=0xe952927e6ff3c66933fa23f228dc74f7eff95fe3
ROLLUP_V1_CONTRACT_ADDR=0x618975654efb35f6674fe9d1afb9f95fa78a31a7
ROLLUP_PROXY_ADMIN_ADDR=0x3a7122f0711822e63aa6218f4db3a6e40f97bdcf
ROLLUP_V2_CONTRACT_ADDR=0x6c5da7ccab84eb7abadbcbe87b3913ccbad0fb9a
ROLLUP_V3_CONTRACT_ADDR=0x9b89bb7a804639bfde8c8d5b5826007988142a38
ROLLUP_V4_CONTRACT_ADDR=0x68427f3169ed36b7b5933446305964f2b3445067
ROLLUP_CONTRACT_ADDR=0x1e44fa332fc0060164061cfedf4d3a1346a9dc38
```

### Testnet

```bash
OWNER=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 PROVER_ADDRESS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 VALIDATORS=0x6B96F1A8D65eDe8AD688716078B3DD79f9BD7323 AMOY_URL=<ETH_RPC> SECRET_KEY=<SECRET_KEY> GAS_PRICE_GWEI=2 yarn deploy -- --network amoy
```

Addresses:

```
USDC_CONTRACT_ADDR=0xa467d2ad4ccae04c310f4aeb919c4f8e5c279d4d
AGGREGATE_BIN_ADDR=0xab728b35532c3a1b6078cd6a14edd412ad241991
AGGREGATE_VERIFIER_ADDR=0x858ac20de1dbcc620d81677721bbe6f9e2f27c15
MINT_BIN_ADDR=0xd603152b1cf72926f3edb7b74f3e03d01145b6b5
MINT_VERIFIER_ADDR=0xb3d051a23cbef5f5c5d14d3259c43b1c771a66f7
BURN_BIN_ADDR=0x60e3c0e0f59ec1db069a82c32ddd8f4f139d67a2
BURN_VERIFIER_ADDR=0x08027543339892d93802550d2a6bb78f7fe7fa30
ROLLUP_V1_CONTRACT_ADDR=0x25e8561ba67e730e43b9c11fee5dd9178b67b639
ROLLUP_PROXY_ADMIN_ADDR=0xbbd9496392636202b941d56eabfbe6903a95504c
ROLLUP_V2_CONTRACT_ADDR=0x29555a798dc9e42cbabfec6f9166c4f872d9d7e2
ROLLUP_V3_CONTRACT_ADDR=0xc0915e1264422d919f45d5bbb775c9d957c19be7
ROLLUP_V4_CONTRACT_ADDR=0xe7f7f14d340b8333e690cc0d4e411e742e98f08c
ROLLUP_CONTRACT_ADDR=0x6d9b36aec2d4c4708c18d12ef9c937051c56a1d7
```

### Mainnet

```bash
OWNER=0x230Dfb03F078B0d5E705F4624fCC915f3126B40f PROVER_ADDRESS=0x5343b904bf837befb2f5a256b0cd5fbf30503d38 VALIDATORS=0x41582701cb3117680687df80bd5a2ca971bda964,0x75eadc4a85ee07e3b60610dc383eab1b27b1c4c1,0x53b385c35d7238d44dfd591eee94fee83f6711de,0x05dc3d71e2a163e6926956bc0769c5cb8a6b9d1a,0x581c5d92e35e51191a982ebd803f92742e3c9fe3,0xbb82aef611b513965371b3d33c4d3b6c8b926f24,0xeacb0b7e37709bafb4204c0c31a2919212049975,0xf9d65db5f8952bee5ea990df79a0032eda0752b7,0x662b7930b201fbe11bcef3cdef6e8f2c8ed4983a,0x68a78d978497b0a87ff8dbeaffae8e68ad4c39dc POLYGON_URL=<ETH_RPC> SECRET_KEY=<SECRET_KEY> yarn deploy -- --network polygon
```

Addresses:

```
USDC_CONTRACT_ADDR=0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359
AGGREGATE_BIN_ADDR=0x31063c00ad62f9090abb9308f4549a1dee4a6362
AGGREGATE_VERIFIER_ADDR=0x9d9fe636a329a07d26b5c5e8411b278462f5f325
MINT_BIN_ADDR=0xe025bb7ce28a4565a890a8d708faf9dd48ea1678
MINT_VERIFIER_ADDR=0xe938b6c17a39e80c7630040df0d2dbe794d42534
BURN_BIN_ADDR=0x4449d93873f7523d1b6cdfaa5a792e0867ca3a17
BURN_VERIFIER_ADDR=0x36e4a9f800e07a4aa6647c83e97f7e47b8028895
ROLLUP_V1_CONTRACT_ADDR=0x470e6986d9a54b498f4fa39ee118d25d52cc0a19
ROLLUP_CONTRACT_ADDR=0x4cbb5041df8d815d752239960fba5e155ba2687e
ROLLUP_PROXY_ADMIN_ADDR=0xe022130f28c4e6ddf1da5be853a185fbeb84d795
```


### Upgrade Rollup contract

Using `yarn upgrade-rollup`, you can upgrade a previously deployed rollup contract to a new version.

Example without a specified network:

```bash
SECRET_KEY=... ROLLUP_CONTRACT_ADDR=<proxy_contract_addr> ROLLUP_PROXY_ADMIN_ADDR=<proxy_admin_contract_addr> yarn upgrade-rollup
```

## Regenerating EVM aggregate proof verifier

To re-generate EVM proof verifier, see [pkg/contracts](/pkg/prover).
