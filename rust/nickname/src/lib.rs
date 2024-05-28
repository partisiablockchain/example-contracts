#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;

use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::context::ContractContext;

/// State of the contract
#[state]
struct ContractState {
    /// AvlTreeMap containing the nicknames
    map: AvlTreeMap<Address, String>,
}

/// Initialize a new Nickname contract.
///
/// # Arguments
///
/// * `_ctx` - the contract context containing information about the sender and the blockchain.
///
/// # Returns
///
/// The initial state of the nickname, with a new AvlTreeMap.
#[init]
fn initialize(_ctx: ContractContext) -> ContractState {
    ContractState {
        map: AvlTreeMap::new(),
    }
}

/// Give a nickname to an address.
///
/// # Arguments
///
/// * `_ctx` - the contract context containing information about the sender and the blockchain.
/// * `state` - the current state of the contract
/// * `key` - the address to receive a nickname
/// * `value` - the nickname
///
/// # Returns
///
/// The state unchanged. Note that AvlTreeMap operations do not create a new state that must be
/// returned. Instead it updates the underlying map in mutable manner.
#[action(shortname = 0x01)]
fn give_nickname(
    _ctx: ContractContext,
    mut state: ContractState,
    key: Address,
    value: String,
) -> ContractState {
    state.map.insert(key, value);

    state
}

/// Remove a nickname from an address.
///
/// # Arguments
///
/// * `_ctx` - the contract context containing information about the sender and the blockchain.
/// * `state` - the current state of the contract
/// * `key` - the address to remove a nickname from
///
/// # Returns
///
/// The state unchanged. Note that AvlTreeMap operations do not create a new state that must be
/// returned. Instead it updates the underlying map in mutable manner.
#[action(shortname = 0x02)]
fn remove_nickname(_ctx: ContractContext, mut state: ContractState, key: Address) -> ContractState {
    state.map.remove(&key);

    state
}
