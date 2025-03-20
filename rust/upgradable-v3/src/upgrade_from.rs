//! Upgrade logic from previous versions of the contract.

use crate::ContractState;
use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::upgrade::ContractHashes;
use read_write_state_derive::ReadWriteState;

/// Upgrade target of the contract, including hashes of the target upgrade code, and the RPC to
/// initialize that code with.
#[derive(ReadWriteState, CreateTypeSpec, Debug, PartialEq, Eq)]
pub struct UpgradeTarget {
    /// Hashes of the new contract code.
    new_contract_hashes: ContractHashes,
    /// Upgrade initialization RPC. Stored in serialized form for validation.
    upgrade_rpc: Vec<u8>,
}

/// Contract state for V2 of the contract.
///
/// Mirror of `ContractState` from `upgradable-v3`.
#[derive(ReadWriteState, CreateTypeSpec, Debug, PartialEq, Eq)]
pub struct UpgradableV2State {
    /// Counter to demonstrate changes in behaviour
    counter: u32,
    /// Contract or account allowed to register updates of the contract.
    upgrade_proposer: Address,
    /// Hashes of the contract code that this contract can be upgraded to.
    upgradable_to: Option<UpgradeTarget>,
}

/// Upgrade contract state from V2 to V3.
///
/// Expects the `increment_amount` to permanently use.
#[upgrade]
pub fn upgrade_from_v2(_context: ContractContext, state: UpgradableV2State) -> ContractState {
    ContractState {
        counter: state.counter,
        increment_amount: 4,
    }
}
