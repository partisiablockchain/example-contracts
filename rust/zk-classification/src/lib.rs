#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;
extern crate pbc_lib;

mod zk_compute;

use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Secret metadata type information.
#[derive(ReadWriteState, ReadWriteRPC, Debug)]
pub enum SecretVarType {
    /// The input model
    #[discriminant(0)]
    InputModel {
        /// The fixed-point scaling conversion ensuring precision in value/threshold comparison.
        /// The thresholds in each (feature, threshold) pair are multiplied by the corresponding
        /// entry, i.e., threshold * scaling\[feature\].
        /// The sample values are multiplied elementwise, i.e., sample\[i\] * scaling\[i\].
        scaling: Vec<u16>,
    },
    /// The input sample
    #[discriminant(1)]
    InputSample {
        /// The identifier of the input model to run sample on
        model_id: SecretVarId,
        /// The address of the receiver of the final result
        result_receiver: Address,
    },
    /// The final result of the evaluation
    #[discriminant(2)]
    InferenceResult {
        /// The address of the owner of the final result
        result_owner: Address,
    },
}

/// Contract state.
///
/// Contains the state of model evaluation on the provided input sample(s). Persisted on-chain.
/// Several input samples can be added to the same contract.
#[state]
pub struct ContractState {
    /// The address of the model owner
    model_owner: Address,
}

/// Contract initialization.
///
/// The model owner initializes the contract, setting the address of the model owner to the address
/// of the contract being called. Model is *not* added at this time.
#[init(zk = true)]
pub fn initialize(context: ContractContext, _zk_state: ZkState<SecretVarType>) -> ContractState {
    ContractState {
        model_owner: context.sender,
    }
}

/// Secret model submission.
///
/// The model owner adds pre-trained model to the contract. Specifies a scaling conversion between fixed
/// points applied to thresholds and sample values to ensure precision when performing comparison.
/// Only the contract owner can add a model, and only one model can be added per contract.
///
/// The model is added as a JSON file consisting of two arrays with keys "internals" and "leaves".
/// The former contains all internal vertices with (feature, threshold) pairs, while the latter
/// contains all leaf vertices with binary classifications. All values stored in the vertices are
/// secret-shared. The thresholds in each (feature, threshold) pair are multiplied by the corresponding
/// entry in the scaling conversion vector provided by the model owner before they are secret-shared.
#[zk_on_secret_input(shortname = 0x40)]
pub fn add_model(
    context: ContractContext,
    state: ContractState,
    _zk_state: ZkState<SecretVarType>,
    scaling_conversion: Vec<u16>,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarType, zk_compute::Model>,
) {
    assert_eq!(
        context.sender, state.model_owner,
        "Only contract creator can add a model"
    );

    let input_def = ZkInputDef::with_metadata(
        Some(inputted_model::SHORTNAME),
        SecretVarType::InputModel {
            scaling: scaling_conversion,
        },
    );

    (state, vec![], input_def)
}

/// Persistence of secret model on the chain.
///
/// Automatically called when a model is confirmed on the chain. Returns the contract state.
#[zk_on_variable_inputted(shortname = 0x40)]
fn inputted_model(
    _context: ContractContext,
    state: ContractState,
    _zk_state: ZkState<SecretVarType>,
    _inputted_model_id: SecretVarId,
) -> ContractState {
    state
}

/// Secret sample submission.
///
/// The sample owner adds input sample to be classified. Specifies the address of the receiver of
/// the final output, i.e., the sample owner can assign the output to themselves or someone else.
///
/// The sample is added as an array of decimal numbers representing each feature value. The
/// scaling conversion provided by the model owner is multiplied onto the sample elementwise
/// before the values are secret-shared.
#[zk_on_secret_input(shortname = 0x41)]
pub fn add_input_sample(
    _context: ContractContext,
    state: ContractState,
    _zk_state: ZkState<SecretVarType>,
    model_id: SecretVarId,
    result_receiver: Address,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarType, zk_compute::Sample>,
) {
    let input_def = ZkInputDef::with_metadata(
        Some(inputted_sample::SHORTNAME),
        SecretVarType::InputSample {
            model_id,
            result_receiver,
        },
    );

    (state, vec![], input_def)
}

/// Persistence of secret input sample on the chain.
///
/// Automatically called when an input sample is confirmed on the chain. Starts the zk computation,
/// evaluating the secret model on the sample. Updates the zk state with the predicted class.
#[zk_on_variable_inputted(shortname = 0x41)]
fn inputted_sample(
    _context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarType>,
    inputted_sample_id: SecretVarId,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    if let SecretVarType::InputSample {
        model_id,
        result_receiver,
    } = zk_state.get_variable(inputted_sample_id).unwrap().metadata
    {
        (
            state,
            vec![],
            vec![zk_compute::evaluate::start(
                model_id,
                inputted_sample_id,
                Some(compute_complete::SHORTNAME),
                &SecretVarType::InferenceResult {
                    result_owner: result_receiver,
                },
            )],
        )
    } else {
        panic!("Could not find sample with given identifier")
    }
}

/// Transfers ownership of the inference result to the contract specified by the input sample owner,
/// keeping it secret from everyone else. The owner of the result can choose to open it at a later
/// time.
///
/// Automatically called when the evaluation of an input sample is complete.
#[zk_on_compute_complete(shortname = 0x42)]
fn compute_complete(
    _context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarType>,
    output_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        output_variables.len(),
        1,
        "Unexpected number of output variables"
    );

    let Some(result_id) = output_variables.first() else {
        panic!("No result")
    };

    if let SecretVarType::InferenceResult { result_owner } =
        zk_state.get_variable(*result_id).unwrap().metadata
    {
        (
            state,
            vec![],
            vec![ZkStateChange::TransferVariable {
                variable: *result_id,
                new_owner: result_owner,
            }],
        )
    } else {
        panic!("Could not find inference result with the given identifier")
    }
}
