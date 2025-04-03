//! Upgrade logic for allowing upgrade.

use crate::ContractState;
use pbc_contract_codegen::upgrade_is_allowed;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::upgrade::ContractHashes;

/// Checks whether the upgrade is allowed.
///
/// This contract allows the [`ContractState::upgrader`] to upgrade the contract at any time.
#[upgrade_is_allowed]
pub fn is_upgrade_allowed(
    context: ContractContext,
    state: ContractState,
    _old_contract_hashes: ContractHashes,
    _new_contract_hashes: ContractHashes,
    _new_contract_rpc: Vec<u8>,
) -> bool {
    context.sender == state.upgrader
}
