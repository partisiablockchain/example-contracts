//! This is an example zero knowledge statistics contract.
//!
//! The contract allows users to input secret data up until a deadline after which any user can
//! start a computation of statistics on the input data.
//! The user submitted data is an age group from { 0-19, 20-39, 40-59, 60- },
//! a gender from { male, female, other } and a favorite color from { red, blue, green, yellow }.
//!
//! The statistics computed are summations of each choice of each variable.

#[macro_use]
extern crate pbc_contract_codegen;

use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_traits::ReadWriteState;
use read_write_state_derive::ReadWriteState;

use crate::statistic_result::StatisticsResult;
use crate::zk_compute::EntryPoint;

mod statistic_result;
mod zk_compute;

/// Public information for a given secret input. Unused for this contract, so a zero-sized struct is used to save space.
#[derive(ReadWriteState, Debug, Clone)]
pub struct PublicInfoForEntry {}

/// Public information for a computed result. Unused for this contract, so a zero-sized struct is used to save space.
#[derive(ReadWriteState, Debug, Clone)]
pub struct PublicInfoForResult {}

/// The public state of the contract.
#[state]
struct StatisticsContractState {
    /// Deadline of the contract after which the statistics can be resulted, and data can no longer be inputted.
    deadline: i64,
    /// Contains the statistics on the input data after their computation.
    result: Option<StatisticsResult>,
}

/// Initial function to bootstrap the contract's state.
///
/// # Parameters
///
///   * `context`: [`ContractContext`] - the contract context containing sender and chain information.
///
///   * `zk_state`: [`ZkState<PublicInfoForEntry>`] - the zk state of the contract.
///
///   * `millis_until_deadline`: [`u64`] - the time in milliseconds until the contract's deadline is reached.
///
/// # Returns
///
/// The new state object of type [`StatisticsContractState`].
///
#[init(zk = true)]
fn initialize(
    context: ContractContext,
    _zk_state: ZkState<PublicInfoForEntry>,
    millis_until_deadline: u64,
) -> StatisticsContractState {
    StatisticsContractState {
        deadline: context.block_production_time + millis_until_deadline as i64,
        result: None,
    }
}

/// Function to input a piece of data to the contract.
///
/// Requires that the deadline has not yet been reached.
///
/// # Parameters
///
///   * `context`: [`ContractContext`] - the contract context containing sender and chain information.
///
///   * `state`: [`StatisticsContractState`] - the current public state of the contract.
///
///   * `zk_state`: [`ZkState<PublicInfoForEntry>`] - the zk state of the contract.
///
/// # Returns
///
/// The unchanged state, an empty event group and a zk input definition.
///
#[zk_on_secret_input(shortname = 0x40, secret_type = "EntryPoint")]
fn add_data(
    context: ContractContext,
    state: StatisticsContractState,
    _zk_state: ZkState<PublicInfoForEntry>,
) -> (
    StatisticsContractState,
    Vec<EventGroup>,
    ZkInputDef<PublicInfoForEntry, EntryPoint>,
) {
    assert!(
        context.block_production_time < state.deadline,
        "Inputting data is not allowed after the deadline. Current time is {}, deadline was {}",
        context.block_production_time,
        state.deadline
    );

    let input_def = ZkInputDef::with_metadata(PublicInfoForEntry {});

    (state, vec![], input_def)
}

/// Function to start the zk computation.
///
/// Requires that the deadline for inputting data has been reached and that the zk calculation state is `Waiting`.
///
/// # Parameters
///
///   * `context`: [`ContractContext`] - the contract context containing sender and chain information.
///
///   * `state`: [`StatisticsContractState`] - the current public state of the contract.
///
///   * `zk_state`: [`ZkState<PublicInfoForEntry>`] - the zk state of the contract.
///
/// # Returns
///
/// The unchanged state, an empty event group and a zk state change that signals the start of the computation.
///
#[action(shortname = 0x01, zk = true)]
fn compute_statistics(
    context: ContractContext,
    state: StatisticsContractState,
    _zk_state: ZkState<PublicInfoForEntry>,
) -> (StatisticsContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert!(
        context.block_production_time >= state.deadline,
        "Cannot start computing statistics before deadline has been passed, deadline is {} ms UTC, current time is {} ms UTC",
        state.deadline,
        context.block_production_time,
    );
    (
        state,
        vec![],
        vec![zk_compute::compute_statistics_start(
            &PublicInfoForResult {},
        )],
    )
}

/// Automatic hook for when the zk computation is finished.
///
/// Sends a signal to open the output variable.
///
/// # Parameters
///
///   * `context`: [`ContractContext`] - the contract context containing sender and chain information.
///
///   * `state`: [`StatisticsContractState`] - the current public state of the contract.
///
///   * `zk_state`: [`ZkState<PublicInfoForEntry>`] - the zk state of the contract.
///
///   * `output_variables`: [`Vec<SecretVarId>`] - the list of zk computation's output variables.
///
/// # Returns
///
/// The unchanged state, an empty event group and a zk state change that signals to open the received variables.
///
#[zk_on_compute_complete]
fn computation_complete(
    _context: ContractContext,
    state: StatisticsContractState,
    _zk_state: ZkState<PublicInfoForEntry>,
    output_variables: Vec<SecretVarId>,
) -> (StatisticsContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    (
        state,
        vec![],
        vec![ZkStateChange::OpenVariables {
            variables: output_variables,
        }],
    )
}

/// Automatic hook for when the zk output variables have been opened.
///
/// Requires that only a single output variable has been opened.
///
/// Reads the computation output, saves it in the contract state and ends the contract.
///
/// # Parameters
///
///   * `context`: [`ContractContext`] - the contract context containing sender and chain information.
///
///   * `state`: [`StatisticsContractState`] - the current public state of the contract.
///
///   * `zk_state`: [`ZkState<PublicInfoForEntry>`] - the zk state of the contract.
///
///   * `opened_variables`: [`Vec<SecretVarId>`] - the list of opened variables.
///
/// # Returns
///
/// The new public state with the result containing the computed statistics, an empty event group and a zk state change that signals that the contract is done.
///
#[zk_on_variables_opened]
fn open_result_variable(
    _context: ContractContext,
    mut state: StatisticsContractState,
    zk_state: ZkState<PublicInfoForEntry>,
    opened_variables: Vec<SecretVarId>,
) -> (StatisticsContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let result = read_result(&zk_state, opened_variables.get(0));

    state.result = Some(result);
    (state, vec![], vec![ZkStateChange::ContractDone])
}

/// Reads the output of the computation.
fn read_result(
    zk_state: &ZkState<PublicInfoForEntry>,
    variable_id: Option<&SecretVarId>,
) -> StatisticsResult {
    let variable_id = *variable_id.unwrap();
    let variable = zk_state.get_variable(variable_id).unwrap();
    let buffer: Vec<u8> = variable.data.clone().unwrap();
    let result = StatisticsResult::state_read_from(&mut buffer.as_slice());

    result
}
