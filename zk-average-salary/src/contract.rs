//! Simple Average Salary contract
//!
//! Average salary is a common multi-party computation example, where several privacy-concious
//! individuals are interested in determining whether they are getting a fair salary, without
//! revealing the salary of any given individual.
//!
//! This implementation works in following steps:
//!
//! 1. Initialization on the blockchain.
//! 2. Receival of multiple secret salaries, using the real zk protocol.
//! 3. Once enough salaries have been received, the contract owner can start the ZK computation.
//! 4. The Zk computation sums all the given salaries together.
//! 5. Once the zk computation is complete, the contract will publicize the the summed variable.
//! 6. Once the summed variable is public, the contract will compute the average and store it in
//!    the state, such that the value can be read by all.
//!
//! NOTE: This contract is missing several features that a production ready contract should
//! possess, including:
//!
//! - An allowlist over salarymen.
//! - Check that each address only sends a single variable.

#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;
extern crate pbc_lib;

use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{CalculationStatus, SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Secret variable metadata. Unused for this contract, so we use a zero-sized struct to save space.
#[derive(ReadWriteState, ReadWriteRPC, Debug)]
struct SecretVarMetadata {
    #[cfg(feature = "plus_metadata")]
    metadata: u32,
}

/// The maximum size of MPC variables.
const BITLENGTH_OF_SECRET_SALARY_VARIABLES: u32 = 32;

/// Number of employees to wait for before starting computation. A value of 2 or below is useless.
const MIN_NUM_EMPLOYEES: u32 = 3;

/// This contract's state
#[state]
struct ContractState {
    /// Address allowed to start computation
    administrator: Address,
    /// Will contain the result (average) when computation is complete
    average_salary_result: Option<u32>,
    /// Will contain the number of employees after starting the computation
    num_employees: Option<u32>,
}

/// Initializes contract
///
/// Note that administrator is set to whoever initializes the contact.
#[init(zk = true)]
fn initialize(ctx: ContractContext, zk_state: ZkState<SecretVarMetadata>) -> ContractState {
    ContractState {
        administrator: ctx.sender,
        average_salary_result: None,
        num_employees: None,
    }
}

/// Adds another salary variable
///
/// The ZkInputDef encodes that the variable should have size [`BITLENGTH_OF_SECRET_SALARY_VARIABLES`].
#[zk_on_secret_input(shortname = 0x40)]
fn add_salary(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata>,
) {
    assert!(
        zk_state
            .secret_variables
            .iter()
            .chain(zk_state.pending_inputs.iter())
            .all(|v| v.owner != context.sender),
        "Each address is only allowed to send one salary variable. Sender: {:?}",
        context.sender
    );
    let input_def = ZkInputDef {
        seal: false,
        metadata: SecretVarMetadata {
            #[cfg(feature = "plus_metadata")]
            metadata: 0x01020304,
        },
        expected_bit_lengths: vec![BITLENGTH_OF_SECRET_SALARY_VARIABLES],
    };
    (state, vec![], input_def)
}

/// Automatically called when a variable is confirmed on chain.
///
/// Unused for this contract, so we do nothing.
#[zk_on_variable_inputted]
fn inputted_variable(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    inputted_variable: SecretVarId,
) -> ContractState {
    state
}

/// Allows the administrator to start the computation of the average salary.
///
/// The averaging computation is automatic beyond this call, involving several steps, as described in the module documentation.
#[action(shortname = 0x01, zk = true)]
fn compute_average_salary(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        context.sender, state.administrator,
        "Only administrator can start computation"
    );
    assert_eq!(
        zk_state.calculation_state,
        CalculationStatus::Waiting,
        "Computation must start from Waiting state, but was {:?}",
        zk_state.calculation_state,
    );

    let num_employees = zk_state.secret_variables.len() as u32;
    assert!(num_employees >= MIN_NUM_EMPLOYEES , "At least {MIN_NUM_EMPLOYEES} employees must have submitted and confirmed their inputs, before starting computation, but had only {num_employees}");

    state.num_employees = Some(num_employees);
    (
        state,
        vec![],
        vec![ZkStateChange::start_computation(vec![SecretVarMetadata {
            #[cfg(feature = "plus_metadata")]
            metadata: 1111,
        }])],
    )
}

/// Automatically called when the computation is completed
///
/// The only thing we do is to instantly open/declassify the output variables.
#[zk_on_compute_complete]
fn sum_compute_complete(
    context: ContractContext,
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

/// Automatically called when a variable is opened/declassified.
///
/// We can now read the sum variable, and compute the average, which will be our final result.
#[zk_on_variables_opened]
fn open_sum_variable(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    opened_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        opened_variables.len(),
        1,
        "Unexpected number of output variables"
    );
    let sum = read_variable_u32_le(&zk_state, opened_variables.get(0));
    let num_employees = state.num_employees.unwrap();
    state.average_salary_result = Some(sum / num_employees);
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
