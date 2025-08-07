#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;

mod zk_compute;

use crate::zk_compute::SecretResponse;
use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_traits::ReadWriteState;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

#[derive(ReadWriteState, ReadWriteRPC, Debug)]
struct SecretVarMetadata {}

/// Public version of the Position struct used in the secret input struct
#[derive(ReadWriteState, CreateTypeSpec, ReadWriteRPC, Clone)]
#[repr(C)]
pub struct Position {
    /// x position
    x: i8,
    /// y position
    y: i8,
}

/// Public version of the secret input struct
#[derive(ReadWriteState, CreateTypeSpec, ReadWriteRPC, Clone)]
#[repr(C)]
pub struct Response {
    /// Age
    pub age: i8,
    /// Height
    pub height: i16,
    /// Position
    pub position: Position,
    /// Wealth
    pub wealth: i128,
}
/// Reads the data from a revealed secret variable
fn read_opened_variable_data<T: ReadWriteState>(
    zk_state: &ZkState<SecretVarMetadata>,
    variable_id: &SecretVarId,
) -> Option<T> {
    let variable = zk_state.get_variable(*variable_id)?;
    variable.open_value()
}

/// State of the contract.
#[derive(Clone)]
#[state]
struct ContractState {
    /// Vector of opened inputs.
    responses: Vec<Response>,
}

/// Initializes contract.
#[init(zk = true)]
fn initialize(ctx: ContractContext, zk_state: ZkState<SecretVarMetadata>) -> ContractState {
    ContractState { responses: vec![] }
}

/// Resets contract state, deleting all received input and secret variables.
#[action(shortname = 0x00, zk = true)]
fn reset_state(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let new_state = ContractState { responses: vec![] };
    let all_variables = zk_state
        .secret_variables
        .iter()
        .chain(zk_state.pending_inputs.iter())
        .map(|(v, _)| v)
        .collect();

    (
        new_state,
        vec![],
        vec![ZkStateChange::DeleteVariables {
            variables_to_delete: all_variables,
        }],
    )
}

/// Add an open input to the list
#[action(shortname = 0x10, zk = true)]
fn open_input(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    response: Response,
) -> ContractState {
    state.responses.push(response);
    state
}

/// Adds a secret input variable of type SecretResponse.
#[zk_on_secret_input(shortname = 0x40, secret_type = "SecretResponse")]
fn secret_input(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata, SecretResponse>,
) {
    let input_def =
        ZkInputDef::with_metadata(Some(output_variables::SHORTNAME), SecretVarMetadata {});

    (state, vec![], input_def)
}

/// Immediately starts a zk computation when the variable input is completed.
#[zk_on_variable_inputted(shortname = 0x41)]
fn output_variables(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    variable_id: SecretVarId,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    (
        state,
        vec![],
        vec![zk_compute::open_but_first_add_300::start(
            variable_id,
            Some(computation_complete::SHORTNAME),
            &SecretVarMetadata {},
        )],
    )
}

/// Immediately opens the output variable of the computation.
#[zk_on_compute_complete(shortname = 0x42)]
fn computation_complete(
    _context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
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

/// Saves the opened variable in state and readies another computation.
#[zk_on_variables_opened]
fn save_opened_variable(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    opened_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let variable_id = opened_variables.first().unwrap();
    let result: Response = read_opened_variable_data(&zk_state, variable_id).unwrap();
    state.responses.push(result);
    (state, vec![], vec![])
}
