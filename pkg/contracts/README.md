# Contracts

Contains the Rust interfaces for the Solidity smart contracts.


## Wallet

Wallet provides a useful utility for working with the USDC contract locally. This is required when developing the deposit/withdraw flows. 

Defaults are designed to work automatically with HardHat/USDC contract.


### Transfer 

Transfer 10 USDC:

```sh
cargo run --bin wallet transfer 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 10
```

### Balance

Owners balance:

```sh
cargo run --bin wallet balance
```

Other users balance:

```sh
cargo run --bin wallet balance 0x70997970C51812dc3A010C7d01b50e0d17dc79C8
```


### Help

```sh
Dev Wallet

Usage: wallet [OPTIONS] <COMMAND>

Commands:
  balance
  transfer  <to> <value>
  help      Print this message or the help of the given subcommand(s)

Options:
  -p, --private-key <PRIVATE_KEY>  [default: ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80]
      --rpc-url <RPC_URL>          [default: http://localhost:8545]
      --rollup-addr <ROLLUP_ADDR>  [default: 2279b7a0a67db372996a5fab50d91eaa73d2ebe6]
      --usdc-addr <USDC_ADDR>      [default: 5fbdb2315678afecb367f032d93f642f64180aa3]
  -h, --help                       Print help
  -V, --version                    Print version
```