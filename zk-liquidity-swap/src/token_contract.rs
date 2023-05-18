//! # Token Contract Detection
//!
//! A contract is detected as a valid Token Contract (similar to
//! [ERC-20](https://ethereum.org/en/developers/docs/standards/tokens/erc-20/)) if the following
//! actions exists where names and types match exactly:
//!
//! ```
//! #[action(shortname=0x01)] transfer(to: Address, amount: u128)
//! #[action(shortname=0x03)] transfer_from(from: Address, to: Address, amount: u128)
//! #[action(shortname=0x05)] approve(spender: Address, amount: u128)
//! ```
//!
//! The root state struct is named TokenState and each of the following state fields exist in the
//! root state struct or a sub-struct that has a 1-1 composition with the root state struct where
//! names and types match exactly:
//!
//! ```
//! balances: Map<Address, u128>
//! name: String
//! symbol: String
//! decimals: u8
//! ```

use pbc_contract_common::address::Address;
use pbc_contract_common::events::EventGroupBuilder;
use pbc_contract_common::shortname::Shortname;

/// Shortname of the token transfer invocation
const SHORTNAME_TRANSFER: Shortname = Shortname::from_u32(0x01);
/// Shortname of the token transfer from invocation
const SHORTNAME_TRANSFER_FROM: Shortname = Shortname::from_u32(0x03);

/// Represents an individual token contract on the blockchain
pub struct TokenContract {
    contract_address: Address,
}

/// Token of the token transfer amounts.
pub type TokenTransferAmount = u128;

impl TokenContract {
    /// Create new token contract representation for the given `contract_address`.
    pub fn at_address(contract_address: Address) -> Self {
        Self { contract_address }
    }

    /// Create an interaction with the `self` token contract, for transferring an `amount` of
    /// tokens from calling contract to `receiver`.
    pub fn transfer(
        &self,
        event_group_builder: &mut EventGroupBuilder,
        receiver: &Address,
        amount: TokenTransferAmount,
    ) {
        event_group_builder
            .call(self.contract_address, SHORTNAME_TRANSFER)
            .argument(*receiver)
            .argument(amount)
            .done();
    }

    /// Create an interaction with the `self` token contract, for transferring an `amount` of
    /// tokens from `sender` to `receiver`. Requires that calling contract have been given an
    /// allowance by `sender`.
    pub fn transfer_from(
        &self,
        event_group_builder: &mut EventGroupBuilder,
        sender: &Address,
        receiver: &Address,
        amount: TokenTransferAmount,
    ) {
        event_group_builder
            .call(self.contract_address, SHORTNAME_TRANSFER_FROM)
            .argument(*sender)
            .argument(*receiver)
            .argument(amount)
            .done();
    }
}
