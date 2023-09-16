#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;

use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::shortname::ShortnameZkComputation;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Uses no additional metadata for the zk secret input.
#[derive(ReadWriteState, ReadWriteRPC, Debug)]
struct SecretVarMetadata {}

/// Input is a 32 bit integer.
const BITLENGTH_OF_SECRET_SALARY_VARIABLES: u32 = 32;

const ZK_COMPUTE: ShortnameZkComputation = ShortnameZkComputation::from_u32(0x61);

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

/// Resets contract state, deleting all received input.
#[action(shortname = 0x00, zk = true)]
fn reset_state(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let new_state = ContractState {
        opened_inputs: vec![],
    };
    (new_state, vec![], vec![])
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
    ZkInputDef<SecretVarMetadata>,
) {
    let input_def = ZkInputDef {
        seal: false,
        metadata: SecretVarMetadata {},
        expected_bit_lengths: vec![BITLENGTH_OF_SECRET_SALARY_VARIABLES],
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
            ZK_COMPUTE,
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

    let result: i32 = read_variable_as_i32(&zk_state, *opened_variables.get(0).unwrap());
    new_state.opened_inputs.push(result);

    (
        new_state,
        vec![],
        vec![ZkStateChange::OutputComplete {
            variables_to_delete: vec![],
        }],
    )
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
