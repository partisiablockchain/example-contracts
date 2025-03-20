#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;
use pbc_contract_codegen::{init, state, upgrade_is_allowed};

use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::upgrade::ContractHashes;

/// Contract state.
#[state]
pub struct ContractState {
    /// Contract or account allowed to upgrade this contract.
    upgrader: Address,
    /// Counter to demonstrate changes in behaviour
    counter: u32,
}

/// Initialize contract with the upgrader address.
#[init]
pub fn initialize(_ctx: ContractContext, upgrader: Address) -> ContractState {
    ContractState {
        counter: 0,
        upgrader,
    }
}

/// Checks whether the upgrade is allowed.
///
/// This contract allows the [`ContractState::upgrader`] to upgrade the contract at any time.
#[upgrade_is_allowed]
pub fn is_upgrade_allowed(
    context: ContractContext,
    state: ContractState,
    _old_contract_hashes: ContractHashes,
    _new_contract_hashes: ContractHashes,
    _new_contract_rpc: Vec<u8>,
) -> bool {
    context.sender == state.upgrader
}

/// Increment the counter by one.
#[action(shortname = 0x01)]
pub fn increment_counter_by_one(
    _context: ContractContext,
    mut state: ContractState,
) -> ContractState {
    state.counter += 1;
    state
}
