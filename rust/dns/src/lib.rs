#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use read_write_state_derive::ReadWriteState;

/// The DNS (Domain Name System) contract contains
/// a mapping from domain names to their respective blockchain adresses.
/// With a DNS contract you can access another smart contract by its domain name,
/// by first looking up the blockchain address for a given domain
/// from the DNS, and then targeting the returned blokchain address.
/// The DNS contract also keeps track domain ownership and
/// thus who can remove and overwrite the domain.
/// The dns-voting-client is an example of a smart contract,
/// that uses the DNS contract.
///
/// A DNS entry.
#[derive(CreateTypeSpec, ReadWriteState, Ord, Eq, PartialEq, PartialOrd)]
pub struct DnsEntry {
    /// The address of the contract.
    address: Address,
    /// The owner of the domain.
    owner: Address,
}

/// The state of the DNS.
#[state]
pub struct DnsState {
    /// A map associating the domains with their respective DNS entry.
    /// Used for saving and retrieving what address corresponds to a given domain, and who owns it.
    records: AvlTreeMap<String, DnsEntry>,
}

impl DnsState {
    /// Find a DNS entry with a given domain
    fn search_domain(&self, domain: &String) -> Option<DnsEntry> {
        self.records.get(domain)
    }

    /// Remove a DNS entry with a given domain
    fn remove_domain(&mut self, domain: &String, sender: Address) {
        if let Some(entry) = self.search_domain(domain) {
            assert_eq!(
                entry.owner, sender,
                "Only the owner of the domain can delete it. Owner: {}, Sender: {}",
                entry.owner, sender
            );

            self.records.remove(domain);
        } else {
            panic!("Could not find domain.")
        };
    }
}

/// Initialize the DNS.
///
/// # Arguments
///
/// * `_ctx` - the contract context containing information about the sender and the blockchain.
///
/// # Returns
///
/// The initial state of the DNS.
///
#[init]
pub fn initialize(ctx: ContractContext) -> DnsState {
    DnsState {
        records: AvlTreeMap::new(),
    }
}

/// Register a domain to a blockchain address, as
/// long as the domain is not taken.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `state` - the current state of the DNS.
/// * `domain` - the domain being registered.
/// * `address` - the address being mapped to the given domain.
///
/// # Returns
///
/// The updated state reflecting the updated DNS.
///
#[action(shortname = 0x01)]
pub fn register_domain(
    ctx: ContractContext,
    mut state: DnsState,
    domain: String,
    address: Address,
) -> DnsState {
    let entry = state.search_domain(&domain);
    assert!(entry.is_none(), "Domain already registered");

    let new_entry = DnsEntry {
        address,
        owner: ctx.sender,
    };

    state.records.insert(domain, new_entry);
    state
}

/// Lookup a domain in the register.
/// Lookup will fail if domain is not found in the register.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and blockchain.
/// * `state` - the current state of the vote.
/// * `domain` - domain to be looked up.
///
/// # Returns
///
/// The state of the DNS, and the address corresponding to the given
/// domain, if the domain is registered.
///
#[action(shortname = 0x02)]
pub fn lookup(
    ctx: ContractContext,
    state: DnsState,
    domain: String,
) -> (DnsState, Vec<EventGroup>) {
    let entry = state.search_domain(&domain);

    assert!(entry.is_some(), "No address found with the given domain");

    let mut event_builder = EventGroup::builder();
    event_builder.return_data(entry.unwrap().address);

    (state, vec![event_builder.build()])
}

/// Remove a domain from the register.
/// Only the owner of the domain can remove it.
/// Will fail if domain is not registered.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `state` - the current state of the DNS.
/// * `domain` - the domain to be removed.
///
/// # Returns
///
/// The updated state reflecting the updated DNS after removing the domain if it was allowed.
///
#[action(shortname = 0x03)]
pub fn remove_domain(ctx: ContractContext, mut state: DnsState, domain: String) -> DnsState {
    state.remove_domain(&domain, ctx.sender);
    state
}

/// Update the address of a registered domain.
/// Only the owner of the domain can update it.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `state` - the current state of the DNS.
/// * `domain` - the domain to update.
/// * `new_address` - the address to be associated with the domain.
///
/// # Returns
///
/// The updated state reflecting the updated DNS after updating the domain, if it was
/// registered and allowed.
///
#[action(shortname = 0x04)]
pub fn update_domain(
    ctx: ContractContext,
    mut state: DnsState,
    domain: String,
    new_address: Address,
) -> DnsState {
    if let Some(entry) = state.search_domain(&domain) {
        assert_eq!(
            entry.owner, ctx.sender,
            "Only the owner of the domain can modify it. Owner: {}, Sender: {}",
            entry.owner, ctx.sender
        );

        state.records.remove(&domain);

        let new_entry = DnsEntry {
            address: new_address,
            owner: ctx.sender,
        };
        state.records.insert(domain, new_entry);
    } else {
        panic!("Could not find domain.")
    };
    state
}
