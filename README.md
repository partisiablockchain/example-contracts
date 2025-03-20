# Partisia Blockchain Example Contracts

The Partisia Blockchain Foundation provides the following reviewed smart contracts,
as examples of real world problems with a blockchain solution.
The easiest way of getting started coding smart contracts is to learn from concrete examples
showing smart contracts solving problems similar to the one you need to solve.

This repository contains multiple example smart contracts as a virtual cargo workspace.

## Overview

Contracts with the `zk` prefix (and `mia-game`) use [Multi-party
computation](https://partisiablockchain.gitlab.io/documentation/smart-contracts/zk-smart-contracts/zk-smart-contracts.html)
for various features. All other contracts are contracts without multi-party
computation.

The three `upgradable` contracts are examples for the upgrade process of smart
contracts:

- `upgradable-v1`: Can be upgraded to a different contract, using a simple
  permission system. This contract is the first version of a smart contract.
- `upgradable-v2`: Can be upgraded to, and can be upgraded from, using a hash
  verification system. This contract is an intermediate version of a smart
  contract.
- `upgradable-v3`: Can be upgraded to, but does not define any way to be
  upgraded from. This contract implements a way to be the final version of the
  smart contract.

## Usage

To compile all the contracts using the partisia-contract tool run:

```bash
cargo partisia-contract build --release
```

To compile a single contract change directory to the specific contract and run the same command.
For example:

```bash
cd token
cargo partisia-contract build --release
```

The compiled wasm/zkwa and abi files are located in

```bash
target/wasm32-unknown-unknown/release
```

To run the test suite, run the following command:

```bash
./run-java-tests.sh
```

To generate the code coverage report, run the following command:

```bash
cargo partisia-contract build --coverage
```

The coverage report will be located in `java-test/target/coverage`
