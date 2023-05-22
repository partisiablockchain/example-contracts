//! This is an example token smart contract.
//!
//! The contract has a constant total supply of tokens.
//! The total supply is initialized together with the contract.
//!
//! Any token owner can then `transfer` tokens to other accounts, or `approve` other accounts to use their tokens.
//! If a Alice has been approved tokens from Bob, then Alice can use `transfer_from` to use Bob's tokens.
//!
//! The contract is inspired by the ERC20 token contract.\
//! <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-20.md>
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;

use create_type_spec_derive::CreateTypeSpec;
use read_write_rpc_derive::ReadWriteRPC;
use std::ops::Add;

use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::sorted_vec_map::SortedVecMap;

mod test;

/// Custom struct for the state of the contract.
///
/// The "state" attribute is attached.
///
/// ### Fields:
///
/// * `name`: [`String`], the name of the token - e.g. "MyToken".\
///
/// * `symbol`: [`String`], the symbol of the token. E.g. "HIX".\
///
/// * `decimals`: [`u8`], the number of decimals the token uses - e.g. 8,
/// means to divide the token amount by `100000000` to get its user representation.\
///
/// * `owner`: [`Address`], owner of the contract.
///
/// * `total_supply`: [`u128`], current amount of tokens for the TokenContract.
///
/// * `balances`: [`SortedVecMap<Address, u128>`], ledger for the accounts associated with the contract.
///
/// * `allowed`: [`SortedVecMap<Address, SortedVecMap<Address, u128>>`], allowance from an owner to a spender.
#[state]
pub struct TokenState {
    name: String,
    decimals: u8,
    symbol: String,
    owner: Address,
    total_supply: u128,
    balances: SortedVecMap<Address, u128>,
    allowed: SortedVecMap<Address, SortedVecMap<Address, u128>>,
}

impl TokenState {
    /// Gets the balance of the specified address.
    ///
    /// ### Parameters:
    ///
    /// * `owner`: The [`Address`] to query the balance of.
    ///
    /// ### Returns:
    ///
    /// An [`u64`] representing the amount owned by the passed address.
    pub fn balance_of(&mut self, owner: Address) -> u128 {
        if !self.balances.contains_key(&owner) {
            self.balances.insert(owner, 0);
        }
        *self.balances.get(&owner).unwrap()
    }

    /// Function to check the amount of tokens that an owner allowed to a spender.
    ///
    /// ### Parameters:
    ///
    /// * `owner`: [`Address`] The address which owns the funds.
    ///
    /// * `spender`: [`Address`] The address which will spend the funds.
    ///
    /// ### Returns:
    ///
    /// A [`u64`] specifying the amount whicher `spender` is still allowed to withdraw from `owner`.
    pub fn allowance(&mut self, owner: Address, spender: Address) -> u128 {
        if !self.allowed.contains_key(&owner) {
            self.allowed.insert(owner, SortedVecMap::new());
        }
        let allowed_from_owner = self.allowed.get_mut(&owner).unwrap();

        if !allowed_from_owner.contains_key(&spender) {
            allowed_from_owner.insert(spender, 0);
        }
        let allowance = allowed_from_owner.get(&spender).unwrap();
        *allowance
    }

    fn update_allowance(&mut self, owner: Address, spender: Address, amount: u128) {
        if !self.allowed.contains_key(&owner) {
            self.allowed.insert(owner, SortedVecMap::new());
        }
        let allowed_from_owner = self.allowed.get_mut(&owner).unwrap();

        allowed_from_owner.insert(spender, amount);
    }
}

/// Initial function to bootstrap the contracts state. Must return the state-struct.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], initial context.
///
/// * `name`: [`String`], the name of the token - e.g. "MyToken".\
///
/// * `symbol`: [`String`], the symbol of the token. E.g. "HIX".\
///
/// * `decimals`: [`u8`], the number of decimals the token uses - e.g. 8,
/// means to divide the token amount by `100000000` to get its user representation.\
///
/// * `total_supply`: [`u128`], current amount of tokens for the TokenContract.
///
/// ### Returns:
///
/// The new state object of type [`TokenContractState`] with an initialized ledger.
#[init]
pub fn initialize(
    ctx: ContractContext,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: u128,
) -> (TokenState, Vec<EventGroup>) {
    let mut balances = SortedVecMap::new();
    balances.insert(ctx.sender, total_supply);

    let state = TokenState {
        name,
        symbol,
        decimals,
        owner: ctx.sender,
        total_supply,
        balances,
        allowed: SortedVecMap::new(),
    };

    (state, vec![])
}

/// Represents the type of a transfer.
#[derive(ReadWriteRPC, CreateTypeSpec)]
pub struct Transfer {
    /// The address to transfer to.
    pub to: Address,
    /// The amount to transfer.
    pub amount: u128,
}

/// Transfers `amount` of tokens to address `to` from the caller.
/// The function throws if the message caller's account
/// balance does not have enough tokens to spend.
/// If the sender's account goes to 0, the sender's address is removed from state.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`TokenContractState`], the current state of the contract.
///
/// * `to`: [`Address`], the address to transfer to.
///
/// * `amount`: [`u128`], amount to transfer.
///
/// ### Returns
///
/// The new state object of type [`TokenContractState`] with an updated ledger.
#[action(shortname = 0x01)]
pub fn transfer(
    context: ContractContext,
    state: TokenState,
    to: Address,
    amount: u128,
) -> (TokenState, Vec<EventGroup>) {
    core_transfer(context.sender, state, to, amount)
}

/// Transfers a bulk of `amount` of tokens to address `to` from the caller.
/// The function throws if the message caller's account
/// balance does not have enough tokens to spend.
/// If the sender's account goes to 0, the sender's address is removed from state.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`TokenContractState`], the current state of the contract.
///
/// * `transfers`: [`Vec[Transfer]`], vector of [the address to transfer to, amount to transfer].
///
/// ### Returns
///
/// The new state object of type [`TokenContractState`] with an updated ledger.
#[action(shortname = 0x02)]
pub fn bulk_transfer(
    context: ContractContext,
    state: TokenState,
    transfers: Vec<Transfer>,
) -> (TokenState, Vec<EventGroup>) {
    let mut new_state = state;
    for t in transfers {
        new_state = core_transfer(context.sender, new_state, t.to, t.amount).0;
    }
    (new_state, vec![])
}

/// Transfers `amount` of tokens from address `from` to address `to`.\
/// This requires that the sender is allowed to do the transfer by the `from`
/// account through the `approve` action.
/// The function throws if the message caller's account
/// balance does not have enough tokens to spend, or if the tokens were not approved.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`TokenContractState`], the current state of the contract.
///
/// * `from`: [`Address`], the address to transfer from.
///
/// * `to`: [`Address`], the address to transfer to.
///
/// * `amount`: [`u128`], amount to transfer.
///
/// ### Returns
///
/// The new state object of type [`TokenContractState`] with an updated ledger.
#[action(shortname = 0x03)]
pub fn transfer_from(
    context: ContractContext,
    state: TokenState,
    from: Address,
    to: Address,
    amount: u128,
) -> (TokenState, Vec<EventGroup>) {
    core_transfer_from(context.sender, state, from, to, amount)
}

/// Transfers a bulk of `amount` of tokens to address `to` from address `from` .\
/// This requires that the sender is allowed to do the transfer by the `from`
/// account through the `approve` action.
/// The function throws if the message caller's account
/// balance does not have enough tokens to spend, or if the tokens were not approved.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`TokenContractState`], the current state of the contract.
///
/// * `from`: [`Address`], the address to transfer from.
///
/// * `transfers`: [`Vec[Transfer]`], vector of [the address to transfer to, amount to transfer].
///
/// ### Returns
///
/// The new state object of type [`TokenContractState`] with an updated ledger.
#[action(shortname = 0x04)]
pub fn bulk_transfer_from(
    context: ContractContext,
    state: TokenState,
    from: Address,
    transfers: Vec<Transfer>,
) -> (TokenState, Vec<EventGroup>) {
    let mut new_state = state;
    for t in transfers {
        new_state = core_transfer_from(context.sender, new_state, from, t.to, t.amount).0;
    }
    (new_state, vec![])
}

/// Allows `spender` to withdraw from the owners account multiple times, up to the `amount`.
/// If this function is called again it overwrites the current allowance with `amount`.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`TokenContractState`], the current state of the contract.
///
/// * `spender`: [`Address`], the address of the spender.
///
/// * `amount`: [`u128`], approved amount.
///
/// ### Returns
///
/// The new state object of type [`TokenContractState`] with an updated ledger.
#[action(shortname = 0x05)]
pub fn approve(
    context: ContractContext,
    state: TokenState,
    spender: Address,
    amount: u128,
) -> (TokenState, Vec<EventGroup>) {
    let mut new_state = state;
    new_state.update_allowance(context.sender, spender, amount);
    (new_state, vec![])
}

/// Transfers `amount` of tokens to address `to` from the caller.
/// The function throws if the message caller's account
/// balance does not have enough tokens to spend.
/// If the sender's account goes to 0, the sender's address is removed from state.
///
/// ### Parameters:
///
/// * `sender`: [`Address`], the sender of the transaction.
///
/// * `state`: [`TokenContractState`], the current state of the contract.
///
/// * `to`: [`Address`], the address to transfer to.
///
/// * `amount`: [`u128`], amount to transfer.
///
/// ### Returns
///
/// The new state object of type [`TokenContractState`] with an updated ledger.
pub fn core_transfer(
    sender: Address,
    state: TokenState,
    to: Address,
    amount: u128,
) -> (TokenState, Vec<EventGroup>) {
    let mut new_state = state;
    let from_amount = new_state.balance_of(sender);
    let o_new_from_amount = from_amount.checked_sub(amount);
    match o_new_from_amount {
        Some(new_from_amount) => {
            new_state.balances.insert(sender, new_from_amount);
        }
        None => {
            panic!("Underflow in transfer - owner did not have enough tokens");
        }
    }
    let to_amount = new_state.balance_of(to);
    new_state.balances.insert(to, to_amount.add(amount));
    if new_state.balance_of(sender) == 0 {
        new_state.balances.remove(&sender);
    };
    (new_state, vec![])
}

/// Transfers `amount` of tokens from address `from` to address `to`.\
/// This requires that the sender is allowed to do the transfer by the `from`
/// account through the `approve` action.
/// The function throws if the message caller's account
/// balance does not have enough tokens to spend, or if the tokens were not approved.
///
/// ### Parameters:
///
/// * `sender`: [`Address`], the sender of the transaction.
///
/// * `state`: [`TokenContractState`], the current state of the contract.
///
/// * `from`: [`Address`], the address to transfer from.
///
/// * `to`: [`Address`], the address to transfer to.
///
/// * `amount`: [`u128`], amount to transfer.
///
/// ### Returns
///
/// The new state object of type [`TokenContractState`] with an updated ledger.
pub fn core_transfer_from(
    sender: Address,
    state: TokenState,
    from: Address,
    to: Address,
    amount: u128,
) -> (TokenState, Vec<EventGroup>) {
    let mut new_state = state;
    let from_allowed = new_state.allowance(from, sender);
    let o_new_allowed_amount = from_allowed.checked_sub(amount);
    match o_new_allowed_amount {
        Some(new_allowed_amount) => {
            new_state.update_allowance(from, sender, new_allowed_amount);
        }
        None => {
            panic!("Underflow in transfer_from - tokens has not been approved for transfer");
        }
    }
    core_transfer(from, new_state, to, amount)
}
