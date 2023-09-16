#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;
extern crate pbc_lib;

use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::shortname::ShortnameZkComputation;
use pbc_contract_common::zk::ZkClosed;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Secret variable metadata.
#[derive(ReadWriteState, ReadWriteRPC, Debug)]
#[repr(C)]
pub struct SecretVarType {}

/// The maximum size of MPC variables.
const BITLENGTH_OF_SECRET_VARIABLES: u32 = 32;

const ZK_COMPUTE_4: ShortnameZkComputation = ShortnameZkComputation::from_u32(0x61);
const ZK_COMPUTE_IDENTITY: ShortnameZkComputation = ShortnameZkComputation::from_u32(0x62);

/// This contract's state
#[state]
pub struct ContractState {
    /// The latest value to be produced and opened.
    pub latest_produced_value: Option<u32>,
}

/// Initializes contract
#[init(zk = true)]
pub fn initialize(ctx: ContractContext, zk_state: ZkState<SecretVarType>) -> ContractState {
    ContractState {
        latest_produced_value: None,
    }
}

/// Requests the opening of the given input
///
/// The ZkInputDef encodes that the variable should have size [`BITLENGTH_OF_SECRET_VARIABLES`].
#[zk_on_secret_input(shortname = 0x40)]
pub fn add_variable(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarType>,
) -> (ContractState, Vec<EventGroup>, ZkInputDef<SecretVarType>) {
    let input_def = ZkInputDef {
        seal: false,
        metadata: SecretVarType {},
        expected_bit_lengths: vec![BITLENGTH_OF_SECRET_VARIABLES],
    };
    (state, vec![], input_def)
}

/// Automatically called when a variable is confirmed on chain.
///
/// Initializes opening.
#[zk_on_variable_inputted]
fn inputted_variable(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarType>,
    inputted_variable: SecretVarId,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    (
        state,
        vec![],
        vec![ZkStateChange::start_computation_with_inputs(
            ZK_COMPUTE_IDENTITY,
            vec![SecretVarType {}],
            vec![inputted_variable],
        )],
    )
}

/// Initializes computation of identity variable.
#[action(shortname = 0x01, zk = true)]
pub fn produce_4(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarType>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    (
        state,
        vec![],
        vec![ZkStateChange::start_computation(
            ZK_COMPUTE_4,
            vec![SecretVarType {}],
        )],
    )
}

/// Automatically called when the computation is completed
///
/// The only thing we do is to instantly open/declassify the output variables.
#[zk_on_compute_complete]
fn sum_compute_complete(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarType>,
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
/// We can now read the result variable and store it in the state.
#[zk_on_variables_opened]
fn open_result_variable(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarType>,
    opened_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let opened_variable = zk_state
        .get_variable(*opened_variables.get(0).unwrap())
        .unwrap();
    state.latest_produced_value = Some(read_variable_u32_le(opened_variable));
    (
        state,
        vec![],
        vec![ZkStateChange::OutputComplete {
            variables_to_delete: vec![],
        }],
    )
}

/// Reads a variable's data as an u32.
fn read_variable_u32_le(sum_variable: &ZkClosed<SecretVarType>) -> u32 {
    let mut buffer = [0u8; 4];
    buffer.copy_from_slice(sum_variable.data.as_ref().unwrap().as_slice());
    <u32>::from_le_bytes(buffer)
}
