#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{CalculationStatus, SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_zk::Sbi32;
use read_write_state_derive::ReadWriteState;

mod zk_compute;

/// Metadata for the contract secret variables.
#[derive(ReadWriteState, Debug)]
#[repr(C)]
struct SecretVarMetadata {
    /// The type of the secret variable. Indicates if the variable is a vote or the number of counted for votes
    variable_type: SecretVarType,
}

/// Type of a secret variable.
#[derive(ReadWriteState, Debug, PartialEq)]
#[repr(u8)]
enum SecretVarType {
    /// The secret variable is a vote.
    Vote = 1,
    /// The secret variable tracks the number of for votes
    CountedForVotes = 2,
}

/// Tracks the result of a vote.
#[derive(ReadWriteState, CreateTypeSpec, Clone)]
struct VoteResult {
    /// Number of 'for' votes.
    votes_for: u32,
    /// Number of 'against' votes.
    votes_against: u32,
    /// Whether the vote passed by a simple majority.
    passed: bool,
}

/// Unit type for [`ContractState::already_voted`] set of users that have voted.
#[derive(ReadWriteState, CreateTypeSpec, Clone)]
struct Unit {}

/// This contract's state
#[state]
struct ContractState {
    /// Address that deployed the contract
    owner: Address,
    /// When the voting stops; at this point all inputs must have been made, and vote counting can
    /// now begin.
    /// Represented as milliseconds since the epoch.
    deadline_voting_time: i64,
    /// A tally that holds the number of votes for, the number of votes against,
    /// and a bool indicating whether the vote passed. It is initialized as None and is
    /// eventually updated to Some(VoteResult) after start_vote_counting is called
    vote_result: Option<VoteResult>,
    /// Maintains the set of voters that have already voted.
    already_voted: AvlTreeMap<Address, Unit>,
}

/// Initializes contract
///
/// # Arguments
/// * `voting_duration_ms` number of milliseconds from contract initialization where voting is
/// open
#[init(zk = true)]
fn initialize(
    ctx: ContractContext,
    _zk_state: ZkState<SecretVarMetadata>,
    voting_duration_ms: u32,
) -> ContractState {
    let deadline_voting_time = ctx.block_production_time + (voting_duration_ms as i64);
    ContractState {
        owner: ctx.sender,
        deadline_voting_time,
        vote_result: None,
        already_voted: AvlTreeMap::new(),
    }
}

/// Casts another vote.
///
/// Can only be used by an address that have not already cast a vote.
#[zk_on_secret_input(shortname = 0x40)]
fn add_vote(
    context: ContractContext,
    mut state: ContractState,
    _zk_state: ZkState<SecretVarMetadata>,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata, Sbi32>,
) {
    assert!(
        context.block_production_time < state.deadline_voting_time,
        "Not allowed to vote after the deadline at {} ms UTC, current time is {} ms UTC",
        state.deadline_voting_time,
        context.block_production_time,
    );
    assert!(
        !state.already_voted.contains_key(&context.sender),
        "Each voter is only allowed to send one vote variable. Sender: {:?}",
        context.sender
    );
    let input_def = ZkInputDef::with_metadata(
        None,
        SecretVarMetadata {
            variable_type: SecretVarType::Vote,
        },
    );
    state.already_voted.insert(context.sender, Unit {});
    (state, vec![], input_def)
}

/// Allows anybody to start the computation of the vote.
///
/// The vote computation is automatic beyond this call, involving several steps, as described in the module documentation.
///
/// NOTE: This ignores any pending inputs
#[action(shortname = 0x01, zk = true)]
fn start_vote_counting(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert!(
        context.block_production_time >= state.deadline_voting_time,
        "Vote counting cannot start before specified starting time {} ms UTC, current time is {} ms UTC",
        state.deadline_voting_time,
        context.block_production_time,
    );
    assert_eq!(
        zk_state.calculation_state,
        CalculationStatus::Waiting,
        "Vote counting must start from Waiting state, but was {:?}",
        zk_state.calculation_state,
    );

    (
        state,
        vec![],
        vec![zk_compute::count_for_votes_start(
            Some(SHORTNAME_COUNTING_COMPLETE),
            &SecretVarMetadata {
                variable_type: SecretVarType::CountedForVotes,
            },
        )],
    )
}

/// Automatically called when the computation is completed
///
/// The only thing we do is to instantly open/declassify the output variables.
#[zk_on_compute_complete(shortname = 0x42)]
fn counting_complete(
    _context: ContractContext,
    state: ContractState,
    _zk_state: ZkState<SecretVarMetadata>,
    output_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    (
        state,
        vec![],
        vec![ZkStateChange::OpenVariables {
            variables: output_variables,
        }],
    )
}

/// Automatically called when a variable is opened/declassified.
///
/// We can now read the for and against variables, and compute the result
#[zk_on_variables_opened]
fn open_sum_variable(
    _context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    opened_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        opened_variables.len(),
        1,
        "Unexpected number of output variables"
    );
    let votes_for = read_variable_u32_le(&zk_state, opened_variables.first());
    let total_votes = zk_state
        .secret_variables
        .iter()
        .filter(|(_, x)| x.metadata.variable_type == SecretVarType::Vote)
        .count();
    let votes_against = (total_votes as u32) - votes_for;

    let vote_result = determine_result(votes_for, votes_against);
    state.vote_result = Some(vote_result);

    (state, vec![], vec![ZkStateChange::ContractDone])
}

/// Reads a variable's data as an u32.
fn read_variable_u32_le(
    zk_state: &ZkState<SecretVarMetadata>,
    sum_variable_id: Option<&SecretVarId>,
) -> u32 {
    let sum_variable_id = *sum_variable_id.unwrap();
    let sum_variable = zk_state.get_variable(sum_variable_id).unwrap();
    let mut buffer = [0u8; 4];
    buffer.copy_from_slice(sum_variable.data.as_ref().unwrap().as_slice());
    <u32>::from_le_bytes(buffer)
}

/// Determines the result of the vote via standard majority decision on inputs the number of votes
/// for and against.
fn determine_result(votes_for: u32, votes_against: u32) -> VoteResult {
    let passed = votes_against < votes_for;
    VoteResult {
        votes_for,
        votes_against,
        passed,
    }
}
