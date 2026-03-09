#![doc = include_str!("../README.md")]
// Allow for the warning in the README.
#![allow(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeSet;
use pbc_contract_common::context::ContractContext;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

mod off_chain;
mod replicated_secret_sharing;
mod signing_orchestration;
mod task_queue;

use crate::signing_orchestration::{PreprocessConfig, SigningComputationState, NUM_ENGINES};
/// Reexporting wrapped functions to make native coverage runner work
pub use signing_orchestration::{
    __pbc_autogen__mul_check_one_complete_wrapped, __pbc_autogen__mul_check_one_report_wrapped,
    __pbc_autogen__mul_check_two_complete_wrapped, __pbc_autogen__mul_check_two_report_wrapped,
    __pbc_autogen__pre_prep_check_complete_wrapped, __pbc_autogen__pre_prep_check_report_wrapped,
    __pbc_autogen__prep_complete_wrapped, __pbc_autogen__prep_report_wrapped,
    __pbc_autogen__sign_complete_wrapped, __pbc_autogen__sign_report_wrapped,
    __pbc_autogen__upload_engine_pub_key_wrapped, __pbc_autogen__upload_pub_key_share_wrapped,
};
use task_queue::EngineIndex;

/// Engine configuration
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
pub struct EngineConfig {
    /// Blockchain address of the engine. Used by contract to verify transactions.
    address: Address,
}

/// State of the contract.
#[state]
pub struct ContractState {
    /// Owner of the contract. Able to reset the preprocessing if it gets stuck
    pub(crate) owner: Address,
    /// Engine configurations
    pub(crate) engines: Vec<EngineConfig>,
    /// State of the signing computations.
    pub(crate) signing_computation_state: SigningComputationState,
    /// Set of users that are allowed to request message signatures.
    pub(crate) signing_users: AvlTreeSet<Address>,
}

impl ContractState {
    /// Engine index for the given [`Address`].
    fn assert_engine(&self, addr: &Address) -> EngineIndex {
        for engine_index in 0..self.engines.len() {
            let address = self.engines.get(engine_index).map(|c| c.address);
            match address {
                Some(address) if &address == addr => {
                    return engine_index as EngineIndex;
                }
                _ => {}
            }
        }
        panic!("Caller is not one of the engines");
    }
}

/// Initialize contract with the given engine configurations.
///
/// ## RPC Arguments
///
/// - `engines`: Configurations for all engines that serve the contract.
/// - `preprocess_config`: Configuration of the pre-processing phase.
/// - `signing_users`: Set of accounts that may request signing of a message.
#[init]
pub fn initialize(
    ctx: ContractContext,
    engines: Vec<EngineConfig>,
    preprocess_config: PreprocessConfig,
    signing_users: Vec<Address>,
) -> ContractState {
    if engines.len() != NUM_ENGINES as usize {
        panic!("Expected {} engines. Got {}.", NUM_ENGINES, engines.len());
    }
    ContractState {
        owner: ctx.sender,
        engines,
        signing_computation_state: SigningComputationState::new(preprocess_config),
        signing_users: signing_users.into_iter().collect(),
    }
}

/// Request a message to be signed in MPC by the allocated execution engines. The signature will
/// be placed in the state once finished.
///
/// Only accounts present in the [`ContractState::signing_users`] field may call this invocation.
///
/// ## RPC Arguments
///
/// - `message`: Message to be signed.
#[action(shortname = 0x01)]
pub fn sign_message(
    ctx: ContractContext,
    mut state: ContractState,
    message: Vec<u8>,
) -> ContractState {
    assert!(
        state.signing_users.contains(&ctx.sender),
        "Not a signing user"
    );
    state
        .signing_computation_state
        .sign_message(message, ctx.sender, ctx.current_transaction);

    state
}

/// Reset the preprocessing and signing queues in case an engine has lost its preprocessing shares
/// or an engine is behaving maliciously, and the work queues are stuck.
///
/// Can only be called by the owner of the contract.
#[action(shortname = 0x20)]
pub fn reset_preprocessing(ctx: ContractContext, mut state: ContractState) -> ContractState {
    assert_eq!(
        ctx.sender, state.owner,
        "Only the owner can reset the contract"
    );
    state.signing_computation_state.reset_preprocessing();
    state
}
