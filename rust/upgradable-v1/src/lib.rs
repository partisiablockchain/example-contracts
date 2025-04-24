#![doc = include_str!("../README.md")]

mod upgrade_to;

#[macro_use]
extern crate pbc_contract_codegen;
use pbc_contract_codegen::{init, state};

use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;

/// Contract state.
#[state]
pub struct ContractState {
    /// Contract or account allowed to upgrade this contract.
    pub upgrader: Address,
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

/// Increment the counter by one.
#[action(shortname = 0x01)]
pub fn increment_counter_by_one(
    _context: ContractContext,
    mut state: ContractState,
) -> ContractState {
    state.counter += 1;
    state
}
