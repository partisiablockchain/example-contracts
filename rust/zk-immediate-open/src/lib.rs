#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;

mod zk_compute;

use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_zk::Sbi32;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Uses no additional metadata for the zk secret input.
#[derive(ReadWriteState, ReadWriteRPC, Debug)]
struct SecretVarMetadata {}

/// State of the contract.
#[state]
struct ContractState {
    /// Vector of opened inputs.
    opened_inputs: Vec<i32>,
}

/// Initializes contract.
#[init(zk = true)]
fn initialize(ctx: ContractContext, zk_state: ZkState<SecretVarMetadata>) -> ContractState {
    ContractState {
        opened_inputs: vec![],
    }
}

/// Resets contract, deleting all received input and secret variables.
#[action(shortname = 0x00, zk = true)]
fn reset_contract(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let new_state = ContractState {
        opened_inputs: vec![],
    };
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

/// Adds a secret input variable.
#[zk_on_secret_input(shortname = 0x40)]
fn secret_input(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata, Sbi32>,
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
        vec![zk_compute::identity::start(
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
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    opened_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let mut new_state = state;

    let result: i32 = read_variable_as_i32(&zk_state, *opened_variables.first().unwrap());
    new_state.opened_inputs.push(result);

    (new_state, vec![], vec![])
}

fn read_variable_as_i32(
    zk_state: &ZkState<SecretVarMetadata>,
    sum_variable_id: SecretVarId,
) -> i32 {
    let sum_variable = zk_state.get_variable(sum_variable_id).unwrap();
    let mut buffer = [0u8; 4];
    buffer.copy_from_slice(sum_variable.data.as_ref().unwrap().as_slice());
    <i32>::from_le_bytes(buffer)
}
