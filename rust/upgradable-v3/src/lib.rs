#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;

use pbc_contract_common::context::ContractContext;

mod upgrade_from;

/// Contract state.
#[state]
pub struct ContractState {
    /// Counter to demonstrate changes in behaviour
    counter: u32,
    /// How much to increment the [`ContractState::counter`] for every invocation.
    increment_amount: u32,
}

/// Initialize contract with the upgrade_proposer address.
#[init]
pub fn initialize(_ctx: ContractContext) -> ContractState {
    ContractState {
        counter: 0,
        increment_amount: 5,
    }
}

/// Increment the counter.
#[action(shortname = 0x01)]
pub fn increment_counter(_context: ContractContext, mut state: ContractState) -> ContractState {
    state.counter += state.increment_amount;
    state
}
