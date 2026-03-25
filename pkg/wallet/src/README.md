# Wallet

## Mint

These commands allow you to mint USDC in dev from our dummy USDC contract.

### Dev

```sh
cargo run --bin wallet transfer <deposit_addr> 100
```


### Prenet

```sh
cargo run --bin wallet -- --private-key <private_key> --usdc-addr=0x206fcb3bea972c5cd6b044160b8b0691fb4aff57 --rpc-url=https://polygon-amoy.g.alchemy.com/v2/9e_9NcJQ4rvg9RCsW2l7dqdbHw0VHBCf transfer <deposit_addr> 10
```
