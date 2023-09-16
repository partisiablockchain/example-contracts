//! # MPC20 invocation helper
//!
//! Mini-library for creating interactions with [MPC20
//! contracts](https://partisiablockchain.gitlab.io/documentation/smart-contracts/integration/mpc-20-token-contract.html),
//! as defined by the [MPC20
//! standard](https://partisiablockchain.gitlab.io/documentation/smart-contracts/integration/mpc-20-token-contract.html)
//! based on the [ERC-20](https://ethereum.org/en/developers/docs/standards/tokens/erc-20/)
//! standard.
//!
//! Assumes that the target contract possesses actions where the shortname and arguments matches
//! the following:
//!
//! ```ignore
//! #[action(shortname=0x01)] transfer(to: Address, amount: u128);
//! #[action(shortname=0x03)] transfer_from(from: Address, to: Address, amount: u128);
//! #[action(shortname=0x05)] approve(spender: Address, amount: u128);
//! ```
//!
//! The root state struct is named TokenState and each of the following state fields exist in the
//! root state struct or a sub-struct that has a 1-1 composition with the root state struct where
//! names and types match exactly:
//!
//! ```ignore
//! balances: Map<Address, u128>
//! name: String
//! symbol: String
//! decimals: u8
//! ```

use pbc_contract_common::address::Address;
use pbc_contract_common::events::EventGroupBuilder;
use pbc_contract_common::shortname::Shortname;

/// Shortname of the MPC20 token transfer invocation
const SHORTNAME_TRANSFER: Shortname = Shortname::from_u32(0x01);

/// Shortname of the MPC20 token transfer from invocation
const SHORTNAME_TRANSFER_FROM: Shortname = Shortname::from_u32(0x03);

/// Represents an individual token contract on the blockchain
pub struct MPC20Contract {
    contract_address: Address,
}

/// Token transfer amounts for the token contract.
pub type TokenTransferAmount = u128;

impl MPC20Contract {
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
