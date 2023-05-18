//! This is an example auction smart contract.
//!
//! The auction sells tokens of one type for another (can be the same token type).
//!
//! The contract works by escrowing bids as well as the tokens for sale.
//! This is done through `transfer` calls to the token contracts with
//! callbacks ensuring that the transfers were successful.
//! If a bid is not the current highest bid the transferred bidding tokens can
//! be claimed during any phase.
//!
//! The auction has a set `duration`. After this duration the auction no longer accepts bids and can
//! be executed by anyone. Once `execute` has been called the contract moves the tokens for sale
//! into the highest bidders claims and the highest bid into the contract owners claims.
//!
//! In the bidding phase any account can call `bid` on the auction which makes a token `transfer`
//! from the bidder to the contract. Once the transfer is done the contract updates its
//! highest bidder accordingly.
//!
//! The contract owner also has the ability to `cancel` the contract during the bidding phase.
//! If cancel is called the highest bid is taken out of escrow such that the highest bidder can
//! claim it again. The same is done for the tokens for sale which the contract owner
//! then can claim.
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::{Address, AddressType, Shortname};
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use read_write_rpc_derive::{ReadRPC, WriteRPC};
use read_write_state_derive::ReadWriteState;

mod tests;

/// Custom struct for bids.
///
/// ### Fields:
///
/// * `bidder`: [`Address`], the address of the bidder.
///
/// * `amount`: [`u128`], the bid amount.
#[derive(ReadRPC, WriteRPC, ReadWriteState, CreateTypeSpec)]
#[cfg_attr(test, derive(PartialEq, Eq, Clone, Debug))]
pub struct Bid {
    bidder: Address,
    amount: u128,
}

/// Custom struct for TokenClaims used by the contracts claim-map.
///
/// ### Fields:
///
/// * `tokens_for_bidding`: [`u128`], The claimable tokens for bidding.
///
/// * `tokens_for_sale`: [`u128`], The claimable tokens for sale.
#[derive(ReadWriteState, CreateTypeSpec)]
#[cfg_attr(test, derive(PartialEq, Eq, Clone, Debug))]
pub struct TokenClaim {
    tokens_for_bidding: u128,
    tokens_for_sale: u128,
}

//// Constants for the different phases of the contract.

type ContractStatus = u8;
const CREATION: ContractStatus = 0;
const BIDDING: ContractStatus = 1;
const ENDED: ContractStatus = 2;
const CANCELLED: ContractStatus = 3;

/// Token contract actions
#[inline]
fn token_contract_transfer() -> Shortname {
    Shortname::from_u32(0x01)
}

#[inline]
fn token_contract_transfer_from() -> Shortname {
    Shortname::from_u32(0x03)
}

/// Custom struct for the state of the contract.
///
/// The "state" attribute is attached.
///
/// ### Fields:
///
/// * `contract_owner`: [`Address`], the owner of the contract as well as the person selling tokens.
///
/// * `start_time`: [`i64`], the start time in millis UTC.
///
/// * `end_time`: [`i64`], the end time in millis UTC.
///
/// * `token_amount_for_sale`: [`u128`], the amount of tokens for sale.
///
/// * `token_for_sale`: [`Address`], the address of the token sold by the contract.
///
/// * `token_for_bidding`: [`Address`], the address of the token used for bids.
///
/// * `highest_bidder`: [`Bid`], the current highest `Bid`.
///
/// * `reserve_price`: [`u128`], the reserve price (minimum cost of the tokens for sale).
///
/// * `min_increment`: [`u128`], the minimum increment of each bid.
///
/// * `claim_map`: [`SortedVecMap<Address, TokenClaim>`], the map of all claimable tokens.
///
/// * `status`: [`u8`], the status of the contract.
#[state]
#[cfg_attr(test, derive(Clone, PartialEq, Eq, Debug))]
pub struct AuctionContractState {
    contract_owner: Address,
    start_time_millis: i64,
    end_time_millis: i64,
    token_amount_for_sale: u128,
    token_for_sale: Address,
    token_for_bidding: Address,
    highest_bidder: Bid,
    reserve_price: u128,
    min_increment: u128,
    claim_map: SortedVecMap<Address, TokenClaim>,
    status: ContractStatus,
}

impl AuctionContractState {
    /// Add a token claim to the `claim_map` of the contract.
    ///
    /// ### Parameters:
    ///
    /// * `bidder`: The [`Address`] of the bidder.
    ///
    /// * `additional_claim`: The additional [`TokenClaim`] that the `bidder` can claim.
    ///
    fn add_to_claim_map(&mut self, bidder: Address, additional_claim: TokenClaim) {
        if !self.claim_map.contains_key(&bidder) {
            self.claim_map.insert(
                bidder,
                TokenClaim {
                    tokens_for_bidding: 0,
                    tokens_for_sale: 0,
                },
            );
        }

        let mut value = self.claim_map.get_mut(&bidder).unwrap();
        value.tokens_for_bidding += additional_claim.tokens_for_bidding;
        value.tokens_for_sale += additional_claim.tokens_for_sale;
    }
}

/// Initial function to bootstrap the contracts state.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], initial context.
///
/// * `token_amount_for_sale`: [`u128`], the amount of tokens that is going to be sold.
///
/// * `token_for_sale`: [`Address`], the address of the token for sale.
///
/// * `token_for_bidding`: [`Address`], the address of the token used for bidding.
///
/// * `reserve_price`: [`u128`], the reserve price (minimum cost of the tokens for sale).
///
/// * `min_increment`: [`u128`], the minimum increment of each bid.
///
/// * `auction_duration_hours`: [`u32`], the duration of the auction in hours.
///
/// ### Returns:
///
/// The new state object of type [`AuctionContractState`] with the initial state being
/// [`CREATION`].
#[init]
pub fn initialize(
    ctx: ContractContext,
    token_amount_for_sale: u128,
    token_for_sale: Address,
    token_for_bidding: Address,
    reserve_price: u128,
    min_increment: u128,
    auction_duration_hours: u32,
) -> (AuctionContractState, Vec<EventGroup>) {
    if token_for_sale.address_type != AddressType::PublicContract {
        panic!("Tried to create a contract selling a non publicContract token");
    }
    if token_for_bidding.address_type != AddressType::PublicContract {
        panic!("Tried to create a contract buying a non publicContract token");
    }
    let duration_millis = i64::from(auction_duration_hours) * 60 * 60 * 1000;
    let end_time_millis = ctx.block_production_time + duration_millis;
    let state = AuctionContractState {
        contract_owner: ctx.sender,
        start_time_millis: ctx.block_production_time,
        end_time_millis,
        token_amount_for_sale,
        token_for_sale,
        token_for_bidding,
        highest_bidder: Bid {
            bidder: ctx.sender,
            amount: 0,
        },
        reserve_price,
        min_increment,
        claim_map: SortedVecMap::new(),
        status: CREATION,
    };

    (state, vec![])
}

/// Action for starting the contract. The function throws an error if the caller isn't the `contract_owner`
/// or the contracts `status` isn't `STARTING`.
/// The contract is started by creating a transfer event from the `contract_owner`
/// to the contract of the tokens being sold as well as a callback to `start_callback`.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`AuctionContractState`], the current state of the contract.
///
/// ### Returns
///
/// The unchanged state object of type [`AuctionContractState`].
#[action(shortname = 0x01)]
pub fn start(
    context: ContractContext,
    state: AuctionContractState,
) -> (AuctionContractState, Vec<EventGroup>) {
    if context.sender != state.contract_owner {
        panic!("Start can only be called by the creator of the contract");
    }
    if state.status != CREATION {
        panic!("Start should only be called while setting up the contract");
    }
    // Create transfer event to contract for the token_for_sale
    // transfer should callback to start_callback (1)

    // Builder

    let mut event_group = EventGroup::builder();

    event_group.with_callback(SHORTNAME_START_CALLBACK).done();

    event_group
        .call(state.token_for_sale, token_contract_transfer_from())
        .argument(context.sender)
        .argument(context.contract_address)
        .argument(state.token_amount_for_sale)
        .done();

    (state, vec![event_group.build()])
}

/// Callback for starting the contract. If the transfer event was successful the `status`
/// is updated to `BIDDING`. If the transfer event failed the callback panics.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the contractContext for the callback.
///
/// * `callback_ctx`: [`CallbackContext`], the callbackContext.
///
/// * `state`: [`AuctionContractState`], the current state of the contract.
///
/// ### Returns
///
/// The new state object of type [`AuctionContractState`].
#[callback(shortname = 0x02)]
pub fn start_callback(
    ctx: ContractContext,
    callback_ctx: CallbackContext,
    state: AuctionContractState,
) -> (AuctionContractState, Vec<EventGroup>) {
    let mut new_state = state;
    if !callback_ctx.success {
        panic!("Transfer event did not succeed for start");
    }
    new_state.status = BIDDING;
    (new_state, vec![])
}

/// Action for bidding on the auction. The function always makes a transfer event
/// to the token for bidding contract. On callback `bid_callback` is called to actually update
/// the state.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`AuctionContractState`], the current state of the contract.
///
/// * `bid_amount`: [`u128`], the amount of tokens in the bid.
///
/// ### Returns
///
/// The unchanged state object of type [`AuctionContractState`].
#[action(shortname = 0x03)]
pub fn bid(
    context: ContractContext,
    state: AuctionContractState,
    bid_amount: u128,
) -> (AuctionContractState, Vec<EventGroup>) {
    // Potential new bid, create the transfer event
    // transfer(auctionContract, bid_amount)

    let bid: Bid = Bid {
        bidder: context.sender,
        amount: bid_amount,
    };

    let mut event_group = EventGroup::builder();
    event_group
        .call(state.token_for_bidding, token_contract_transfer_from())
        .argument(context.sender)
        .argument(context.contract_address)
        .argument(bid_amount)
        .done();
    event_group
        .with_callback(SHORTNAME_BID_CALLBACK)
        .argument(bid)
        .done();
    (state, vec![event_group.build()])
}

/// Callback from bidding. If the transfer event was successful the `bid` will be compared
/// to the current highest bid and the claim map is updated accordingly.
/// If the transfer event fails the state is unchanged.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the contractContext for the callback.
///
/// * `callback_ctx`: [`CallbackContext`], the callbackContext.
///
/// * `state`: [`AuctionContractState`], the current state of the contract.
///
/// * `bid`: [`Bid`], the bid containing information as to who the bidder was and which
/// amount was bid.
///
/// ### Returns
///
/// The new state object of type [`AuctionContractState`].
#[callback(shortname = 0x04)]
pub fn bid_callback(
    ctx: ContractContext,
    callback_ctx: CallbackContext,
    state: AuctionContractState,
    bid: Bid,
) -> (AuctionContractState, Vec<EventGroup>) {
    let mut new_state = state;
    if !callback_ctx.success {
        panic!("Transfer event did not succeed for bid");
    } else if new_state.status != BIDDING
        || ctx.block_production_time >= new_state.end_time_millis
        || bid.amount < new_state.highest_bidder.amount + new_state.min_increment
        || bid.amount < new_state.reserve_price
    {
        // transfer succeeded, since we are no longer accepting bids we add
        // this to the claim map so the sender can get his money back
        // if the bid was too small we also add it to the claim map
        new_state.add_to_claim_map(
            bid.bidder,
            TokenClaim {
                tokens_for_bidding: bid.amount,
                tokens_for_sale: 0,
            },
        );
    } else {
        // bidding phase and a new highest bid
        let prev_highest_bidder = new_state.highest_bidder;
        // update highest bidder
        new_state.highest_bidder = bid;
        // move previous highest bidders coin into the claim map
        new_state.add_to_claim_map(
            prev_highest_bidder.bidder,
            TokenClaim {
                tokens_for_bidding: prev_highest_bidder.amount,
                tokens_for_sale: 0,
            },
        );
    }
    (new_state, vec![])
}

/// Action for claiming tokens. Can be called at any time during the auction. Only the highest
/// bidder and the owner of the contract cannot get their escrowed tokens.
/// If there is any available tokens for the sender in the claim map the contract creates
/// appropriate transfer calls for both the token for sale and the token for bidding. The entry in
/// the claim map is then set to 0 for both token types.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`AuctionContractState`], the current state of the contract.
///
/// ### Returns
///
/// The new state object of type [`AuctionContractState`].
#[action(shortname = 0x05)]
pub fn claim(
    context: ContractContext,
    state: AuctionContractState,
) -> (AuctionContractState, Vec<EventGroup>) {
    let mut new_state = state;
    let opt_claimable = new_state.claim_map.get(&context.sender);
    match opt_claimable {
        None => (new_state, vec![]),
        Some(claimable) => {
            let mut event_group = EventGroup::builder();
            if claimable.tokens_for_bidding > 0 {
                event_group
                    .call(new_state.token_for_bidding, token_contract_transfer())
                    .argument(context.sender)
                    .argument(claimable.tokens_for_bidding)
                    .done();
            }
            if claimable.tokens_for_sale > 0 {
                event_group
                    .call(new_state.token_for_sale, token_contract_transfer())
                    .argument(context.sender)
                    .argument(claimable.tokens_for_sale)
                    .done();
            }
            new_state.claim_map.insert(
                context.sender,
                TokenClaim {
                    tokens_for_bidding: 0,
                    tokens_for_sale: 0,
                },
            );
            (new_state, vec![event_group.build()])
        }
    }
}

/// Action for executing the auction. Panics if the block time is earlier than the contracts
/// end time or if the current status is not `BIDDING`. When the contract is executed the status
/// is changed to `ENDED`, and the highest bidder will be able to claim the sold tokens.
/// Similarly the contract owner is able to claim the amount of bidding tokens that the highest
/// bidder bid.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`AuctionContractState`], the current state of the contract.
///
/// ### Returns
///
/// The new state object of type [`AuctionContractState`].
#[action(shortname = 0x06)]
pub fn execute(
    context: ContractContext,
    state: AuctionContractState,
) -> (AuctionContractState, Vec<EventGroup>) {
    let mut new_state = state;
    if context.block_production_time < new_state.end_time_millis {
        panic!("Tried to execute the auction before auction end block time");
    } else if new_state.status != BIDDING {
        panic!("Tried to execute the auction when the status isn't Bidding");
    } else {
        new_state.status = ENDED;
        new_state.add_to_claim_map(
            new_state.contract_owner,
            TokenClaim {
                tokens_for_bidding: new_state.highest_bidder.amount,
                tokens_for_sale: 0,
            },
        );
        new_state.add_to_claim_map(
            new_state.highest_bidder.bidder,
            TokenClaim {
                tokens_for_bidding: 0,
                tokens_for_sale: new_state.token_amount_for_sale,
            },
        );
        (new_state, vec![])
    }
}

/// Action for cancelling the auction. Panics if the caller is not the contract owner, the
/// block time is later than the contracts end time, or if the status is not `BIDDING`.
/// When the contract is cancelled the status is changed to `CANCELLED`, and the highest bidder
/// will be able to claim the amount of tokens he bid. Similarly the contract owner is
/// able to claim the tokens previously for sale.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`AuctionContractState`], the current state of the contract.
///
/// ### Returns
///
/// The new state object of type [`AuctionContractState`].
#[action(shortname = 0x07)]
pub fn cancel(
    context: ContractContext,
    state: AuctionContractState,
) -> (AuctionContractState, Vec<EventGroup>) {
    let mut new_state = state;
    if context.sender != new_state.contract_owner {
        panic!("Only the contract owner can cancel the auction");
    } else if context.block_production_time >= new_state.end_time_millis {
        panic!("Tried to cancel the auction after auction end block time");
    } else if new_state.status != BIDDING {
        panic!("Tried to cancel the auction when the status isn't Bidding");
    } else {
        new_state.status = CANCELLED;
        new_state.add_to_claim_map(
            new_state.highest_bidder.bidder,
            TokenClaim {
                tokens_for_bidding: new_state.highest_bidder.amount,
                tokens_for_sale: 0,
            },
        );
        new_state.add_to_claim_map(
            new_state.contract_owner,
            TokenClaim {
                tokens_for_bidding: 0,
                tokens_for_sale: new_state.token_amount_for_sale,
            },
        );
        (new_state, vec![])
    }
}
