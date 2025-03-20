# Upgradable v2

Upgradable example WASM smart contract with hash-based upgrade check.

The [`UpgradableV2State`] contains:

- The address of the account or contract that is allowed to register hashes for
  upgraded versions of the contract.
- The registered upgrade hashes, and the registered upgrade RPC.

Anybody is allowed to upgrade the contract, as long as they use the correct
hashes and RPC. This avoids storing the binaries as blobs in the governance
layer that manages upgrades.

Contract can upgrade from `upgradable-v1`.

## About upgrade governance

This contract is an example, and does not reflect what good upgrade logic for a
contract should look like. Please read documentation page for [upgradable smart
contracts](https://partisiablockchain.gitlab.io/documentation/smart-contracts/upgradable-smart-contracts.html)
for suggestion of how to implement the upgrade governance.