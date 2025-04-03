#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::upgrade::ContractHashes;
use read_write_state_derive::ReadWriteState;

mod upgrade_from;

/// Upgrade target of the contract, including hashes of the target upgrade code, and the RPC to
/// initialize that code with.
#[derive(ReadWriteState, CreateTypeSpec, Debug, PartialEq)]
pub struct UpgradeTarget {
    /// Hashes of the new contract code.
    new_contract_hashes: ContractHashes,
    /// Upgrade initialization RPC. Stored in serialized form for validation.
    upgrade_rpc: Vec<u8>,
}

/// Contract state.
#[state]
pub struct ContractState {
    /// Counter to demonstrate changes in behaviour
    counter: u32,
    /// Contract or account allowed to register updates of the contract.
    upgrade_proposer: Address,
    /// Hashes of the contract code that this contract can be upgraded to.
    upgradable_to: Option<UpgradeTarget>,
}

/// Initialize contract with the upgrade_proposer address.
#[init]
pub fn initialize(_ctx: ContractContext, upgrade_proposer: Address) -> ContractState {
    ContractState {
        counter: 0,
        upgrade_proposer,
        upgradable_to: None,
    }
}

/// Increment the counter by two.
#[action(shortname = 0x01)]
pub fn increment_counter_by_two(
    _context: ContractContext,
    mut state: ContractState,
) -> ContractState {
    state.counter += 2;
    state
}

/// Approves the given contract hashes as a valid target of an upgrade.
#[action(shortname = 0x70)]
pub fn allow_upgrade_to(
    context: ContractContext,
    mut state: ContractState,
    new_contract_hashes: ContractHashes,
    upgrade_rpc: Vec<u8>,
) -> ContractState {
    assert_eq!(
        context.sender, state.upgrade_proposer,
        "The upgrade_proposer is the only address allowed to propose upgrades."
    );
    state.upgradable_to = Some(UpgradeTarget {
        new_contract_hashes,
        upgrade_rpc,
    });
    state
}
