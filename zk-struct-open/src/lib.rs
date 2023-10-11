#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::shortname::ShortnameZkComputation;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_traits::ReadWriteState;
use pbc_zk::{Sbi128, Sbi16, Sbi8, SecretBinary};
use read_write_rpc_derive::{ReadRPC, ReadWriteRPC};
use read_write_state_derive::ReadWriteState;

const ZK_COMPUTE_OPEN_ADD_300: ShortnameZkComputation = ShortnameZkComputation::from_u32(0x61);

#[derive(ReadWriteState, ReadWriteRPC, Debug)]
struct SecretVarMetadata {}

/// Secret position used in the secret input struct
#[derive(CreateTypeSpec, SecretBinary)]
#[allow(dead_code)]
struct SecretPosition {
    /// x position
    x: Sbi8,
    /// y position
    y: Sbi8,
}

/// Secret struct used as the secret input type
#[derive(CreateTypeSpec, SecretBinary)]
#[allow(dead_code)]
struct SecretResponse {
    age: Sbi8,
    height: Sbi16,
    position: SecretPosition,
    wealth: Sbi128,
}

/// Public version of the Position struct used in the secret input struct
#[derive(ReadWriteState, CreateTypeSpec, ReadRPC, Clone)]
#[repr(C)]
pub struct Position {
    /// x position
    x: i8,
    /// y position
    y: i8,
}

/// Public version of the secret input struct
#[derive(ReadWriteState, CreateTypeSpec, ReadRPC, Clone)]
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

impl Response {
    const BITS: u32 = 8 + 16 + 16 + 128;
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
        .map(|v| v.variable_id)
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
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    response: Response,
) -> ContractState {
    let mut new_state = state;
    new_state.responses.push(response);
    new_state
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
    ZkInputDef<SecretVarMetadata>,
) {
    let input_def = ZkInputDef {
        seal: false,
        metadata: SecretVarMetadata {},
        expected_bit_lengths: vec![Response::BITS],
    };

    (state, vec![], input_def)
}

/// Immediately starts a zk computation when the variable input is completed.
#[zk_on_variable_inputted]
fn output_variables(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    variable_id: SecretVarId,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    (
        state,
        vec![],
        vec![ZkStateChange::start_computation_with_inputs(
            ZK_COMPUTE_OPEN_ADD_300,
            vec![SecretVarMetadata {}],
            vec![variable_id],
        )],
    )
}

/// Immediately opens the output variable of the computation.
#[zk_on_compute_complete]
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
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    opened_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let mut new_state = state;
    let variable_id = opened_variables.get(0).unwrap();
    let result: Response = read_opened_variable_data(&zk_state, variable_id).unwrap();
    new_state.responses.push(result);
    (new_state, vec![], vec![])
}
