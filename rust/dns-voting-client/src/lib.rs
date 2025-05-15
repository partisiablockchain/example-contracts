#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use pbc_contract_common::address::{Address, Shortname};
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;

/// The DNS voting client is an example of how the DNS contract can be used.
/// The contract can vote on a voting contract given the domain of the voting contract.
/// The voting client looks up the address
/// of that voting contract and propagates the vote to the voting contracts address.
///
/// The state of the DNS voting client.
#[state]
pub struct DnsVotingClientState {
    /// The address of the DNS.
    dns_address: Address,
}

/// Initialize the DNS voting client.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `dns_address` - The address of the DNS contract.
///
/// # Returns
///
/// The initial state of the DNS voting client.
///
#[init]
pub fn initialize(ctx: ContractContext, dns_address: Address) -> DnsVotingClientState {
    DnsVotingClientState { dns_address }
}

/// Casts a vote on a given voting domain.
/// Creates an event calling the DNS contract, where the address corresponding to the domain is found.
/// Also creates a callback to `vote_callback`.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `state` - the current state of the DNS client.
/// * `voting_domain` - the domain to vote on.
/// * `vote` - The vote to be cast.
///
/// # Returns
///
/// The updated state reflecting the updated DNS voting client.
///
#[action(shortname = 0x01)]
pub fn vote(
    ctx: ContractContext,
    state: DnsVotingClientState,
    voting_domain: String,
    vote: bool,
) -> (DnsVotingClientState, Vec<EventGroup>) {
    let mut event_group = EventGroup::builder();

    event_group
        .call(state.dns_address, Shortname::from_u32(0x02))
        .argument(voting_domain.to_string())
        .with_cost(1000)
        .done();

    event_group
        .with_callback_rpc(vote_callback::rpc(vote))
        .with_cost(1000)
        .done();

    (state, vec![event_group.build()])
}

/// Callback for casting a vote through a domain.
/// This calls the found address of the voting domain, and casts the given vote.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `callback_context` - the context of the callback.
/// * `state` - the current state of the DNS client.
/// * `vote` - The vote to be cast.
///
/// # Returns
///
/// The updated state reflecting the updated DNS voting client.
///
#[callback(shortname = 0x02)]
pub fn vote_callback(
    context: ContractContext,
    callback_context: CallbackContext,
    state: DnsVotingClientState,
    vote: bool,
) -> (DnsVotingClientState, Vec<EventGroup>) {
    let voting_address: Address = callback_context.results.first().unwrap().get_return_data();

    let mut event_group = EventGroup::builder();

    event_group
        .call(voting_address, Shortname::from_u32(0x01))
        .argument(vote)
        .done();

    (state, vec![event_group.build()])
}
