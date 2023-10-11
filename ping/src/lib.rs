#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate core;

use pbc_contract_common::address::Address;
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::{EventGroup, GasCost};

/// This is the state of the contract which is persisted on the chain.
///
/// The #\[state\] attribute generates serialization logic for the struct.
#[state]
pub struct PingContractState {}

/// Initialize the contract.
///
/// ### Parameters
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
#[init]
pub fn initialize(context: ContractContext) -> (PingContractState, Vec<EventGroup>) {
    (PingContractState {}, vec![])
}

/// Pings contract at `destination` to check for its existence or transfer gas.
///
/// `cost` must be at least the network fee.
///
/// `cost` amount of gas will be transferred to `destination`.
/// By specifying `cost` as `None` will send the maximum possible.
///
/// Creates a callback which checks for the existence of the `destination` contract.
/// If this functionality alone is desired, use the minimum possible `cost`.
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///  * `state`: [`PingContractState`] - The current state of the contract.
///  * `destination`: [`Address`] - The destination address of the contract to ping.
///  * `cost`: [`Option<GasCost>`] - How much gas to use for the interaction.
#[action(shortname = 0x01)]
pub fn ping(
    context: ContractContext,
    state: PingContractState,
    destination: Address,
    cost: Option<GasCost>,
) -> (PingContractState, Vec<EventGroup>) {
    let mut event_group_builder = EventGroup::builder();
    event_group_builder.ping(destination, cost);
    event_group_builder
        .with_callback(SHORTNAME_PING_CALLBACK)
        .done();
    (state, vec![event_group_builder.build()])
}

/// Pings contract at `destination` to transfer gas.
///
/// Does not create any callback to check for existence of `destination` contract.
///
/// `cost` must be at least the network fee.
///
/// `cost` amount of gas will be transferred to `destination`.
/// By specifying `cost` as `None` will send the maximum possible.
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///  * `state`: [`PingContractState`] - The current state of the contract.
///  * `destination`: [`Address`] - The destination address of the contract to ping.
///  * `cost`: [`Option<GasCost>`] - How much gas to use for the interaction.
#[action(shortname = 0x02)]
pub fn ping_no_callback(
    context: ContractContext,
    state: PingContractState,
    destination: Address,
    cost: Option<GasCost>,
) -> (PingContractState, Vec<EventGroup>) {
    let mut event_group_builder = EventGroup::builder();
    event_group_builder.ping(destination, cost);
    (state, vec![event_group_builder.build()])
}

/// Checks for contract existence by handling `ping` callback.
///
/// If the callback context of the `ping` call was unsuccessful, the `destination` doesn't exist.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`] - The contract context for the callback.
/// * `callback_context`: [`CallbackContext`] - The context of the callback.
/// * `state`: [`PingContractState`] - The current state of the contract.
///
/// ### Returns
///
/// The updated state object of type [`PingContractState`]
#[callback(shortname = 0x10)]
pub fn ping_callback(
    context: ContractContext,
    callback_context: CallbackContext,
    state: PingContractState,
) -> (PingContractState, Vec<EventGroup>) {
    assert!(
        callback_context.success,
        "No contract found at called address"
    );
    (state, vec![])
}
