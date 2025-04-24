# Upgradable v1

The simplest possible upgradable example contract that retains some amount of
security and usability.

The `UpgradableV1State` contains the address of the account or contract that is
allowed to upgrade it.

Contract can only be upgraded to a different contract, it cannot be upgraded to
itself, or from any other kind of contract.

## About upgrade governance

This contract is an example, and does not reflect what good upgrade logic for a
contract should look like. Please read documentation page for [upgradable smart
contracts](https://partisiablockchain.gitlab.io/documentation/smart-contracts/upgradable-smart-contracts.html)
for suggestion of how to implement the upgrade governance.
