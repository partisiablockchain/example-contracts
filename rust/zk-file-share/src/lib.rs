#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;
extern crate pbc_lib;

use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_zk::Sbi8;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

mod zk_compute;

/// Metadata for secret-shared files.
#[derive(ReadWriteState, ReadWriteRPC, Debug)]
#[repr(C)]
pub struct SecretVarMetadata {}

/// Empty contract state, as all stored files are secret-shared.
#[state]
pub struct CollectionState {}

/// Initializes contract with empty state.
#[init(zk = true)]
pub fn initialize(ctx: ContractContext, zk_state: ZkState<SecretVarMetadata>) -> CollectionState {
    CollectionState {}
}

/// Upload a new file with a specific size of `file_length`.
///
/// `file_length` is the size of the file in *bytes*.
/// Fails if the uploaded file has a different size than `file_length`.
#[zk_on_secret_input(shortname = 0x42)]
pub fn add_file(
    context: ContractContext,
    state: CollectionState,
    zk_state: ZkState<SecretVarMetadata>,
    file_length: u32,
) -> (
    CollectionState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata, Vec<Sbi8>>,
) {
    let input_def = ZkInputDef::with_metadata_and_size(None, SecretVarMetadata {}, file_length * 8);
    (state, vec![], input_def)
}

/// Changes ownership of the secret-shared file with id `file_id`
/// from the sender to `new_owner`.
///
/// Fails if the sender is not the current owner of the referenced file.
#[action(shortname = 0x03, zk = true)]
pub fn change_file_owner(
    ctx: ContractContext,
    state: CollectionState,
    zk_state: ZkState<SecretVarMetadata>,
    file_id: u32,
    new_owner: Address,
) -> (CollectionState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let file_id = SecretVarId::new(file_id);
    let file_owner = zk_state.get_variable(file_id).unwrap().owner;
    assert_eq!(
        file_owner, ctx.sender,
        "Only the owner of the secret file is allowed to change ownership."
    );

    (
        state,
        vec![],
        vec![ZkStateChange::TransferVariable {
            variable: file_id,
            new_owner,
        }],
    )
}

/// Deletes the secret-shared file with id `file_id`.
///
/// Fails if the sender is not the current owner of the secret file.
#[action(shortname = 0x05, zk = true)]
pub fn delete_file(
    ctx: ContractContext,
    state: CollectionState,
    zk_state: ZkState<SecretVarMetadata>,
    file_id: u32,
) -> (CollectionState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let file_id = SecretVarId::new(file_id);
    let file_owner = zk_state.get_variable(file_id).unwrap().owner;
    assert_eq!(
        file_owner, ctx.sender,
        "Only the owner of the secret file is allowed to delete it."
    );

    (
        state,
        vec![],
        vec![ZkStateChange::DeleteVariables {
            variables_to_delete: vec![file_id],
        }],
    )
}
