<img src="./payy.png" alt="Payy Logo" width="200" height="200">

# Payy - ZK Rollup

An Ethereum L2 ZK rollup for privacy preserving and regulatory compliant transactions. 

Here are some highlights:

 - 🚀 Fast - runs in under 3 seconds on an iPhone
 - 🪄 Tiny - UTXO proofs are under 2.8KB
 - ✅ EVM - proofs are compatible with Ethereum 

For a detailed description of our architecture, please [download our whitepaper](https://polybase.github.io/zk-rollup/whitepaper.pdf) or visit our [docs](https://payy.network/docs).


| Module             | Path                                    | Desc                                                            |
|--------------------|-----------------------------------------|-----------------------------------------------------------------|
| Ethereum Contracts | [eth](/eth)                             | Ethereum smart contracts to verify state transitions and proofs |
| Contracts          | [pkg/prover](/pkg/prover)               | Rust interface to Ethereum smart contracts                      |
| RPC                | [pkg/rpc](/pkg/rpc-server)              | RPC common utilities shared across all RPC services             |
| Smirk              | [pkg/smirk](/pkg/smirk)                 | Sparse merkle tree                                              |
| ZK-Circuits        | [pkg/zk-circuits](/pkg/zk-circuits)     | Halo2 + KZG ZK circuits for proving UTXO, merkle and state transitions      |
| ZK-Primitives      | [pkg/zk-primitives](/pkg/zk-primitives) | ZK primitives used across multiple modules                      |


## Tests

```
cargo test
```

Note: these tests can take a while to run on your laptop (e.g. more than 20 minutes)


## Audit

The ZK-Circuits and Ethereum Contracts have been audited by KALOS.


## Git LFS

We use [Git LFS](https://git-lfs.com/) for storing large files (e.g. srs params).

A one-time setup needs to be done for local development:

  1. Install `git lfs` following the instructions at https://git-lfs.com/
  2. Inside the `zk-rollup` root directory, run the following commands:

  ```bash
  $ git lfs install
  $ git lfs pull
  ```


## Contributing

We appreciate your interest in contributing to our open-source project. Your contributions help improve the project for everyone.

### Code of Conduct

This project adheres to the Contributor Covenant [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [hello@polybaselabs.com](mailto:hello@polybaselabs.com).

### Security vulnerabilities

We take security issues seriously. If you discover a security vulnerability, we appreciate your assistance in disclosing it to us in a responsible manner. Do not report security vulnerabilities through public issues or forums. Instead, send a full report to [hello@polybaselabs.com](mailto:hello@polybaselabs.com). We do not have an official bug bounty program but will reward responsibly disclosed vulnerabilities at our discretion.


### Reporting Bugs

If you find a bug, please report it by [opening an issue](https://github.com/polybase/payy/issues). Include as much detail as possible, including steps to reproduce the issue, the environment in which it occurs, and any relevant screenshots or code snippets.

### Suggesting Enhancements

We appreciate enhancements! To suggest a feature or enhancement, please [open an issue](https://github.com/polybase/payy/issues) with a detailed description of your proposal. Explain why it is needed and how it would benefit the project.

### Submitting Pull Requests

1. Fork the repository
2. Create a new branch (`git checkout -b feature/YourFeature`)
3. Make your changes
4. Commit your changes (`git commit -m 'Add some feature'`)
5. Push to the branch (`git push origin feature/YourFeature`)
6. Open a pull request

### License

By contributing, you agree that your contributions will be licensed under the same license as the project. For more details, see [LICENSE](LICENSE).
