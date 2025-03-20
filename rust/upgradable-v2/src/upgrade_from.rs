//! Upgrade logic from previous versions of the contract.

use crate::ContractState;
use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use read_write_rpc_derive::{ReadRPC, WriteRPC};
use read_write_state_derive::ReadWriteState;

/// Contract state for V1 of the contract.
///
/// This is a mirror of the `ContractState` struct from `upgradable-v1`.
#[derive(ReadWriteState, ReadRPC, WriteRPC, PartialEq, Eq, CreateTypeSpec)]
pub struct UpgradableV1State {
    /// Contract or account allowed to upgrade this contract.
    upgrader: Address,
    /// Counter to demonstrate changes in behaviour
    counter: u32,
}

/// Upgrade contract state from V1 to V2.
#[upgrade]
pub fn upgrade_from_v1(_context: ContractContext, state: UpgradableV1State) -> ContractState {
    ContractState {
        counter: state.counter,
        upgrade_proposer: state.upgrader,
        upgradable_to: None,
    }
}
