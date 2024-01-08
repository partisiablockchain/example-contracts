#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use pbc_contract_common::address::{Address, AddressType, Shortname};
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use pbc_traits::WriteRPC;

const PUB_DEPLOY_ADDRESS: Address = Address {
    address_type: AddressType::SystemContract,
    identifier: [
        0x97, 0xa0, 0xe2, 0x38, 0xe9, 0x24, 0x02, 0x5b, 0xad, 0x14, 0x4a, 0xa0, 0xc4, 0x91, 0x3e,
        0x46, 0x30, 0x8f, 0x9a, 0x4d,
    ],
};

/// Contract state.
///
/// ### Fields:
///
/// * `owner`: [`Address`], the owner of the contract.
/// * `eligible_voters`: [`Vec<Address>`], the list of legal voters.
/// * `voting_contracts`: [`SortedVecMap<u64, Option<Address>>`], A map from proposal ids to voting contracts.
/// * `voting_contract_wasm`: [`Vec<u8>`], bytes of the voting contract wasm.
/// * `voting_contract_abi`: [`Vec<u8>`], bytes of the voting contract abi.
#[state]
pub struct MultiVotingState {
    owner: Address,
    eligible_voters: Vec<Address>,
    voting_contracts: SortedVecMap<u64, Option<Address>>,
    voting_contract_wasm: Vec<u8>,
    voting_contract_abi: Vec<u8>,
}

/// Initial function to create the initial state.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], initial context.
/// * `voting_contract_wasm`: [`Vec<u8>`], wasm bytes of a voting contract.
/// * `voting_contract_abi`: [`Vec<u8>`], abi bytes of a voting contract.
///
/// ### Returns:
/// The initial state of type [`MultiVotingState`].
#[init]
pub fn initialize(
    ctx: ContractContext,
    voting_contract_wasm: Vec<u8>,
    voting_contract_abi: Vec<u8>,
) -> (MultiVotingState, Vec<EventGroup>) {
    let eligible_voters = vec![ctx.sender];
    let state = MultiVotingState {
        owner: ctx.sender,
        eligible_voters,
        voting_contracts: SortedVecMap::new(),
        voting_contract_wasm,
        voting_contract_abi,
    };

    (state, vec![])
}

/// Adds a voter to eligible voters. This voter can then vote on voting contracts. Only the
/// owner of the contract can add voters.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the action call.
/// * `state`: [`MultiVotingState`], the state before the call.
/// * `voter`: [`Address`], the voter to be added.
///
/// ### Returns:
/// The new state of type [`MultiVotingState`].
#[action]
pub fn add_voter(
    ctx: ContractContext,
    mut state: MultiVotingState,
    voter: Address,
) -> (MultiVotingState, Vec<EventGroup>) {
    assert_eq!(ctx.sender, state.owner, "Only owner can add voters");
    let voter_exists = state.eligible_voters.iter().any(|x| *x == voter);
    if voter_exists {
        panic!("Voter already exists");
    }
    state.eligible_voters.push(voter);
    (state, vec![])
}

/// Removes a voter from eligible voters. This voter can no longer vote on voting contracts.
/// Only the owner of the contract can remove voters.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the action call.
/// * `state`: [`MultiVotingState`], the state before the call.
/// * `voter`: [`Address`], the voter to be removed.
///
/// ### Returns:
/// The new state of type [`MultiVotingState`].
#[action]
pub fn remove_voter(
    ctx: ContractContext,
    mut state: MultiVotingState,
    voter: Address,
) -> (MultiVotingState, Vec<EventGroup>) {
    assert_eq!(ctx.sender, state.owner, "Only owner can remove voters");
    let index = state
        .eligible_voters
        .iter()
        .position(|x| *x == voter)
        .expect("Voter does not exist");
    state.eligible_voters.remove(index);
    (state, vec![])
}

/// Deploys a new voting contract with given proposal id. The voting contract is deployed with
/// eligible voters as those who can vote. The address of the new voting contract is computed
/// from the original transaction hash. Only the owner can add new voting contracts, and the
/// proposal id has to be unique.
/// This creates an event to the public deploy contract as well as creates a callback to
/// `add_voting_contract_callback`.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the action call.
/// * `state`: [`MultiVotingState`], the state before the call.
/// * `p_id`: [`u64`], the proposal id of the new voting contract.
///
/// ### Returns:
/// The new state of type [`MultiVotingState`].
#[action]
pub fn add_voting_contract(
    ctx: ContractContext,
    mut state: MultiVotingState,
    p_id: u64,
    deadline: i64,
) -> (MultiVotingState, Vec<EventGroup>) {
    assert_eq!(ctx.sender, state.owner, "Only owner can add contracts");
    if state.voting_contracts.contains_key(&p_id) {
        panic!("Proposal id already exists");
    }

    state.voting_contracts.insert(p_id, None);

    let voting_address = Address {
        address_type: AddressType::PublicContract,
        identifier: ctx.original_transaction.bytes[12..32].try_into().unwrap(),
    };

    let mut event_group = EventGroup::builder();

    event_group
        .call(PUB_DEPLOY_ADDRESS, Shortname::from_u32(1))
        .argument(state.voting_contract_wasm.clone())
        .argument(state.voting_contract_abi.clone())
        .argument(create_voting_init_bytes(
            p_id,
            &state.eligible_voters,
            deadline,
        ))
        .done();

    event_group
        .with_callback(SHORTNAME_ADD_VOTING_CONTRACT_CALLBACK)
        .with_cost(1000)
        .argument(p_id)
        .argument(voting_address)
        .done();

    (state, vec![event_group.build()])
}

/// Callback for adding a new voting contract. If the deployment was unsuccessful the entry in
/// `voting_contracts` is deleted. If it instead was successful, an empty invocation is made to
/// the new contract to check if it really has been deployed. A new callback to
/// `voting_contract_exists_callback` is also created.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the call.
/// * `callback_ctx`: [`CallbackContext`], the context of the callback.
/// * `state`: [`MultiVotingState`], the state before the call.
/// * `p_id`: [`u64`], the proposal id of the new voting contract.
/// * `voting_address`: [`Address`], the address of the the new voting contract.
///
/// ### Returns:
/// The new state of type [`MultiVotingState`].
#[callback(shortname = 0x01)]
pub fn add_voting_contract_callback(
    ctx: ContractContext,
    callback_ctx: CallbackContext,
    mut state: MultiVotingState,
    p_id: u64,
    voting_address: Address,
) -> (MultiVotingState, Vec<EventGroup>) {
    if !callback_ctx.results[0].succeeded {
        state.voting_contracts.remove(&p_id);
        (state, vec![])
    } else {
        let mut event_group = EventGroup::builder();

        event_group.ping(voting_address, None);
        event_group
            .with_callback(SHORTNAME_VOTING_CONTRACT_EXISTS_CALLBACK)
            .argument(p_id)
            .argument(voting_address)
            .done();

        (state, vec![event_group.build()])
    }
}

/// Callback for checking if a voting contract has been deployed successfully. If it is the
/// address is inserted into `voting_contracts`. If it is not the entry is deleted instead.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the call.
/// * `callback_ctx`: [`CallbackContext`], the context of the callback.
/// * `state`: [`MultiVotingState`], the state before the call.
/// * `p_id`: [`u64`], the proposal id of the new voting contract.
/// * `voting_address`: [`Address`], the address of the the new voting contract.
///
/// ### Returns:
/// The new state of type [`MultiVotingState`].
#[callback(shortname = 0x02)]
pub fn voting_contract_exists_callback(
    ctx: ContractContext,
    callback_ctx: CallbackContext,
    mut state: MultiVotingState,
    p_id: u64,
    voting_address: Address,
) -> (MultiVotingState, Vec<EventGroup>) {
    if !callback_ctx.results[0].succeeded {
        state.voting_contracts.remove(&p_id);
    } else {
        state.voting_contracts.insert(p_id, Some(voting_address));
    }
    (state, vec![])
}

fn create_voting_init_bytes(proposal_id: u64, voters: &Vec<Address>, deadline: i64) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![0xff, 0xff, 0xff, 0xff, 0x0f];
    WriteRPC::rpc_write_to(&proposal_id, &mut bytes).unwrap();
    WriteRPC::rpc_write_to(voters, &mut bytes).unwrap();
    WriteRPC::rpc_write_to(&deadline, &mut bytes).unwrap();
    bytes
}
