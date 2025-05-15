<div align="center">

![Partisia Blockchain](https://partisiablockchain.com/wp-content/uploads/2023/10/chartAsset-1.svg)

# Example Contracts

[Official Repository](https://gitlab.com/partisiablockchain/language/example-contracts)
| [Rustdoc](https://partisiablockchain.gitlab.io/language/example-contracts/access_control/index.html)
| [PBC Documentation](https://partisiablockchain.gitlab.io/documentation/)

</div>

[Partisia Blockchain Foundation](https://partisiablockchain.com/) provides
the following reviewed smart contracts, as examples of real world problems with
a blockchain solution. The easiest way of getting started coding smart
contracts is to learn from concrete examples showing smart contracts solving
problems similar to the one you need to solve.

Smart Contracts are written in the [Rust programming
language](https://rust-lang.org/), using the [Paritisa Blockchain Contract
SDK](https://gitlab.com/partisiablockchain/language/contract-sdk/). Tests of contract
functionality is written in Java, and uses the
[Junit contract testing framework](https://gitlab.com/partisiablockchain/language/junit-contract-test/).

## Contracts Overview

Contracts of this repository demonstrate the four following feature sets:

- **Basic Smart Contracts**: These demonstrate how to [develop a standard smart
  contract](https://partisiablockchain.gitlab.io/documentation/smart-contracts/introduction-to-smart-contracts.html)
  in Rust for deployment to Partisia Blockchain.
- **ZK (Multi-Party Computation)**: These demonstrate how to use Partisia
  Blockchain's unique [Multi-party
  computations](https://partisiablockchain.gitlab.io/documentation/smart-contracts/zk-smart-contracts/zk-smart-contracts.html)
  capability to enhance privacy on Web3, while retaining full auditability.
- **Upgradable**: These show how to [upgrade contracts on Partisia
  Blockchain](https://partisiablockchain.gitlab.io/documentation/smart-contracts/upgradable-smart-contracts.html),
  and demonstrate possible governance models.
- **Off-Chain**: These demonstrate the unique _Off-Chain_ feature, which allows
  you to implement off-chain servers directly in your on-chain contract, and
  enables easy versioning of both on-chain and off-chain systems.

The **Basic** contracts are:

- [`petition`](./rust/petition): Implements a petition that users can sign.
- [`voting`](./rust/voting): Implements majority open ballot vote.
- [`ping`](./rust/ping): Demonstrates contract-to-contract interaction with
  a contract that sends a lot of interactions around.
- [`dns`](./rust/dns): Implements a simplified DNS, which allows users to name
  individual contract addresses, much like the world-wide DNS system names IP
  addresses.
- [`dns-voting-client`](./rust/dns-voting-client): Example contract using [the
  DNS contract](../dns) to route invocations to contracts by using the domain
  name of the wanted contract.
- [`nickname`](./rust/nickname): An even more simplified DNS.
- [`access-control`](./rust/access-control): Showcases how access control
  systems can be implemented in smart contracts.

The **ZK (Multi-Party Computation)** contracts are:

- [`zk-average-salary`](./rust/zk-average-salary): Implements an algorithm for
  determining the average of several users' input values in secret.
- [`zk-second-price-auction`](./rust/zk-second-price-auction): Implements
  a second-price auction, such that users can bid in secret. Only the second
  highest bid is revealed.
- [`mia-game`](./rust/mia-game): Implements the rules of the Mia game.
  Demonstrates how to generate randomness on-chain.
- [`zk-classification`](./rust/zk-classification): Implements evaluation of
  a pre-trained [decision tree](https://en.wikipedia.org/wiki/Decision_tree) on
  a user-provided input sample. Demonstrates Partisia Blockchain's
  applicability for machine learning while retaining confidentiality.
- [`zk-file-share`](./rust/zk-file-share): Contract for storing uploaded files
  as secret-shared variables, to act as a file-sharing service of secret files.
- [`zk-statistics`](./rust/zk-statistics): Demonstrates how to perform
  statistics on secret-shared data.
- [`zk-voting-simple`](./rust/zk-voting-simple): Implements majority closed
  ballot votes, where all votes are secret. Only the vote result is published.
- [`zk-immediate-open`](./rust/zk-immediate-open): Simplified contract that
  immediately opens any secret-shared variables uploaded. Demonstrates the
  life-time flow of the `zk_` invocations.
- [`zk-multi-functional`](./rust/zk-multi-functional): Simplified contract that
  demonstrates a contract can implement several multi-party computations.
- [`zk-struct-open`](./rust/zk-struct-open): Simplified contract that shows how
  to use Rust `struct`s in smart contracts.

The **Off-Chain** contracts are:

- [`off-chain-secret-sharing`](./rust/off-chain-secret-sharing): Contract that allows user to store secret shares, distributed between a set of nodes.
- [`off-chain-publish-randomness`](./rust/off-chain-publish-randomness):
  Contract that automatically provides random bytes to the blockchain, created
  through a distributed process.

The three **Upgradable** contracts are examples for the upgrade process of smart
contracts:

- [`upgradable-v1`](./rust/upgradable-v1): Can be upgraded to a different contract, using a simple
  permission system. This contract is the first version of a smart contract.
- [`upgradable-v2`](./rust/upgradable-v2): Can be upgraded to, and can be upgraded from, using a hash
  verification system. This contract is an intermediate version of a smart
  contract.
- [`upgradable-v3`](./rust/upgradable-v3): Can be upgraded to, but does not define any way to be
  upgraded from. This contract implements a way to be the final version of the
  smart contract.

## Usage

All smart contracts can be compiled using the [Cargo Partisia Contract](https://gitlab.com/partisiablockchain/language/cargo-partisia-contract) tool from the `rust` directory:

```bash
cargo pbc build --release
```

The `--release` argument ensures that contracts are minimized. Resulting
contract `.pbc` files can be found in the `rust/target/wasm32-unknown-unknown/release` folder, and can be
 [directly deployed to Partisia Blockchain](https://partisiablockchain.gitlab.io/documentation/smart-contracts/compile-and-deploy-contracts.html).

Individual contracts can be compiled directly from the respective contract's
directory.

### Testing

The smart contract test suite is run by using the following script:

```bash
./run-java-tests.sh -bc
```

The `-b` argument ensure that contracts are rebuilt, while the `-c` argument
results in a coverage report, located at `contract-java-test/target/coverage`.
