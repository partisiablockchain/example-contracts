#![allow(deprecated)]
#![cfg(test)]
use pbc_contract_common::address::{Address, AddressType, ShortnameCallback};
use pbc_contract_common::context::{CallbackContext, ContractContext, ExecutionResult};
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::Hash;

use crate::{
    bid, bid_callback, cancel, claim, execute, initialize, start, start_callback,
    AuctionContractState, Bid, Shortname, TokenClaim, BIDDING, CANCELLED, ENDED,
};

fn create_ctx(sender: Address, block_time: i64) -> ContractContext {
    let hash: Hash = Hash {
        bytes: [
            0u8, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1,
        ],
    };
    let ctx: ContractContext = ContractContext {
        contract_address: get_contract_address(),
        sender,
        block_time,
        block_production_time: block_time * 3_600_000,
        current_transaction: hash.clone(),
        original_transaction: hash,
    };
    ctx
}

fn get_owner_address() -> Address {
    Address {
        address_type: AddressType::Account,
        identifier: [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    }
}

fn get_contract_address() -> Address {
    Address {
        address_type: AddressType::PublicContract,
        identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    }
}

fn get_currency_token_address() -> Address {
    Address {
        address_type: AddressType::PublicContract,
        identifier: [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
    }
}

fn get_commodity_token_address() -> Address {
    Address {
        address_type: AddressType::PublicContract,
        identifier: [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
    }
}

fn get_bidder_address() -> Address {
    Address {
        address_type: AddressType::Account,
        identifier: [
            0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x0b, 0x1d,
        ],
    }
}

fn get_third_party_address() -> Address {
    Address {
        address_type: AddressType::Account,
        identifier: [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5],
    }
}

fn create_callback_ctx(success: bool) -> CallbackContext {
    let ctx: CallbackContext = CallbackContext {
        success,
        results: vec![ExecutionResult {
            succeeded: success,
            return_data: vec![],
        }],
    };
    ctx
}

fn initialize_contract() -> (AuctionContractState, Vec<EventGroup>) {
    let sender = get_owner_address();
    let commodity_token = get_commodity_token_address();
    let currency_token = get_currency_token_address();
    let ctx = create_ctx(sender, 2);
    initialize(
        ctx,
        100_000,
        commodity_token,
        currency_token,
        1_000,
        100,
        100,
    )
}

#[test]
pub fn test_initialize() {
    let sender = get_owner_address();
    let commodity_token = get_commodity_token_address();
    let currency_token = get_currency_token_address();
    let ctx = create_ctx(sender, 2);
    let (state, events) = initialize(
        ctx,
        100_000,
        commodity_token,
        currency_token,
        1_000,
        100,
        100,
    );
    assert_eq!(0, events.len());
    assert_eq!(0, state.status);
    assert_eq!(sender, state.contract_owner);
    assert_eq!(commodity_token, state.token_for_sale);
    assert_eq!(currency_token, state.token_for_bidding);
    let highest_bidder = state.highest_bidder;
    assert_eq!(sender, highest_bidder.bidder);
    assert_eq!(0, highest_bidder.amount);
    assert_eq!(100_000, state.token_amount_for_sale);
    assert_eq!(7_200_000, state.start_time_millis);
    assert_eq!(102 * 3_600_000, state.end_time_millis);
    assert_eq!(100, state.min_increment);
    assert_eq!(1_000, state.reserve_price);
    assert_eq!(0, state.claim_map.len());
}

#[test]
#[should_panic]
pub fn test_initialize_wrong_commodity() {
    let sender = get_owner_address();
    let commodity_token = Address {
        address_type: AddressType::Account,
        identifier: [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
    };
    let currency_token = get_currency_token_address();
    let ctx = create_ctx(sender, 2);
    let (state, events) = initialize(
        ctx,
        100_000,
        commodity_token,
        currency_token,
        1_000,
        100,
        100,
    );
}

#[test]
#[should_panic]
pub fn test_initialize_wrong_currency() {
    let sender = get_owner_address();
    let commodity_token = get_commodity_token_address();
    let currency_token = Address {
        address_type: AddressType::Account,
        identifier: [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
    };
    let ctx = create_ctx(sender, 2);
    let (state, events) = initialize(
        ctx,
        100_000,
        commodity_token,
        currency_token,
        1_000,
        100,
        100,
    );
}

#[test]
pub fn test_start() {
    let (state, _) = initialize_contract();
    let sender = get_owner_address();
    let ctx = create_ctx(sender, 3);
    let (start_state, start_events) = start(ctx, state.clone());
    assert_eq!(start_state, state);
    assert_eq!(start_events.len(), 1);
    let transfer_event = start_events.get(0).unwrap();
    let mut expected = EventGroup::builder();
    expected
        .call(state.token_for_sale, Shortname::from_u32(3))
        .argument(sender)
        .argument(get_contract_address())
        .argument(100_000u128)
        .done();
    expected
        .with_callback(ShortnameCallback::from_u32(2))
        .done();
    assert_eq!(*transfer_event, expected.build());
}

#[test]
#[should_panic]
pub fn test_start_not_creation() {
    let (mut state, _) = initialize_contract();
    let sender = get_owner_address();
    state.status = 1;
    let ctx = create_ctx(sender, 3);
    start(ctx, state);
}

#[test]
#[should_panic]
pub fn test_start_not_owner() {
    let (state, _) = initialize_contract();
    let sender = get_third_party_address();
    let ctx = create_ctx(sender, 3);
    start(ctx, state);
}

#[test]
pub fn test_start_callback() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let (start_state, _) = start(start_ctx, init_state);
    let callback_ctx = create_callback_ctx(true);
    let start_ctx_2 = create_ctx(owner, 4);
    let (start_callback_state, events) = start_callback(start_ctx_2, callback_ctx, start_state);
    assert_eq!(start_callback_state.status, BIDDING);
    assert_eq!(events.len(), 0);
}

#[test]
#[should_panic]
pub fn test_start_callback_transfer_unsuccessful() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let (start_state, _) = start(start_ctx, init_state);
    let callback_ctx = create_callback_ctx(false);
    let start_ctx_2 = create_ctx(owner, 4);
    start_callback(start_ctx_2, callback_ctx, start_state);
}

#[test]
pub fn test_bid() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let (start_state, _) = start(start_ctx, init_state);
    let callback_ctx = create_callback_ctx(true);
    let start_ctx_2 = create_ctx(owner, 4);
    let (start_callback_state, _) = start_callback(start_ctx_2, callback_ctx, start_state);
    let bidder = get_bidder_address();
    let bid_ctx = create_ctx(bidder, 5);
    let (bid_state, events) = bid(bid_ctx, start_callback_state.clone(), 10);
    assert_eq!(bid_state, start_callback_state);
    assert_eq!(events.len(), 1);
    let bid_event = events.get(0).unwrap();
    let mut expected_event = EventGroup::builder();
    expected_event
        .call(get_currency_token_address(), Shortname::from_u32(3))
        .argument(get_bidder_address())
        .argument(get_contract_address())
        .argument(10u128)
        .done();
    expected_event
        .with_callback(ShortnameCallback::from_u32(4))
        .argument(bidder)
        .argument(10u128)
        .done();
    assert_eq!(*bid_event, expected_event.build());
}

#[test]
pub fn test_bid_callback_new_highest_bid() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let start_callback_ctx = create_callback_ctx(true);
    let (start_callback_state, _) = start_callback(start_ctx, start_callback_ctx, init_state);
    let bidder = get_bidder_address();
    let bid_ctx = create_ctx(bidder, 4);
    let bid_callback_ctx = create_callback_ctx(true);
    let bid = Bid {
        bidder,
        amount: 1000,
    };
    assert_eq!(start_callback_state.claim_map.len(), 0);
    let (bid_callback_state, bid_callback_events) =
        bid_callback(bid_ctx, bid_callback_ctx, start_callback_state, bid.clone());
    assert_eq!(bid_callback_events.len(), 0);
    // previous bid is added to claim map (owner, currency: 0)
    assert_eq!(bid_callback_state.claim_map.len(), 1);
    let claim_map_entry = bid_callback_state.claim_map.get(&owner);
    assert!(claim_map_entry.is_some());
    assert_eq!(
        *claim_map_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 0,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(bid_callback_state.highest_bidder, bid);
}

#[test]
pub fn test_bid_callback_not_bidding() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    // contract not started yet
    let bidder = get_bidder_address();
    let bid_ctx = create_ctx(bidder, 4);
    let bid_callback_ctx = create_callback_ctx(true);
    let bid = Bid {
        bidder,
        amount: 1000,
    };
    assert_eq!(init_state.claim_map.len(), 0);
    let (bid_callback_state, bid_callback_events) =
        bid_callback(bid_ctx, bid_callback_ctx, init_state, bid);
    assert_eq!(bid_callback_events.len(), 0);
    // bid is added to claim map (bidder, currency: 0)
    assert_eq!(bid_callback_state.claim_map.len(), 1);
    let claim_map_entry = bid_callback_state.claim_map.get(&bidder);
    assert!(claim_map_entry.is_some());
    assert_eq!(
        *claim_map_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 1000,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(
        bid_callback_state.highest_bidder,
        Bid {
            bidder: owner,
            amount: 0,
        }
    );
}

#[test]
pub fn test_bid_callback_end_time_reached() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let start_callback_ctx = create_callback_ctx(true);
    let (start_callback_state, _) = start_callback(start_ctx, start_callback_ctx, init_state);
    let bidder = get_bidder_address();
    // contract init at block time 2 with duration 100
    let bid_ctx = create_ctx(bidder, 102);
    let bid_callback_ctx = create_callback_ctx(true);
    let bid = Bid {
        bidder,
        amount: 1000,
    };
    assert_eq!(start_callback_state.claim_map.len(), 0);
    let (bid_callback_state, bid_callback_events) =
        bid_callback(bid_ctx, bid_callback_ctx, start_callback_state, bid);
    assert_eq!(bid_callback_events.len(), 0);
    // bid is added to claim map (bidder, currency: 0)
    assert_eq!(bid_callback_state.claim_map.len(), 1);
    let claim_map_entry = bid_callback_state.claim_map.get(&bidder);
    assert!(claim_map_entry.is_some());
    assert_eq!(
        *claim_map_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 1000,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(
        bid_callback_state.highest_bidder,
        Bid {
            bidder: owner,
            amount: 0,
        }
    );
}

#[test]
pub fn test_bid_callback_multiple_claimable_bids() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let start_callback_ctx = create_callback_ctx(true);
    let (start_callback_state, _) = start_callback(start_ctx, start_callback_ctx, init_state);
    let bidder = get_bidder_address();
    // contract init at block time 2 with duration 100
    let bid_ctx = create_ctx(bidder, 102);
    let bid_callback_ctx = create_callback_ctx(true);
    let bid = Bid {
        bidder,
        amount: 1000,
    };
    assert_eq!(start_callback_state.claim_map.len(), 0);
    let (bid_callback_state, _) =
        bid_callback(bid_ctx, bid_callback_ctx, start_callback_state, bid.clone());
    let bid_ctx = create_ctx(bidder, 102);
    let bid_callback_ctx = create_callback_ctx(true);
    let (bid2_callback_state, bid2_callback_events) =
        bid_callback(bid_ctx, bid_callback_ctx, bid_callback_state, bid);
    assert_eq!(bid2_callback_events.len(), 0);
    // bid is added to claim map (bidder, currency: 0)
    assert_eq!(bid2_callback_state.claim_map.len(), 1);
    let claim_map_entry = bid2_callback_state.claim_map.get(&bidder);
    assert!(claim_map_entry.is_some());
    assert_eq!(
        *claim_map_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 2000,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(
        bid2_callback_state.highest_bidder,
        Bid {
            bidder: owner,
            amount: 0,
        }
    );
}

#[test]
pub fn test_bid_callback_not_highest_bid_cause_increment() {
    let (mut init_state, _) = initialize_contract();
    init_state.reserve_price = 0;
    init_state.min_increment = 100;
    assert_eq!(init_state.highest_bidder.amount, 0);
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let start_callback_ctx = create_callback_ctx(true);
    let (start_callback_state, _) = start_callback(start_ctx, start_callback_ctx, init_state);
    let bidder = get_bidder_address();
    let bid_ctx = create_ctx(bidder, 101);
    let bid_callback_ctx = create_callback_ctx(true);
    let bid = Bid { bidder, amount: 99 };
    assert_eq!(start_callback_state.claim_map.len(), 0);
    let (bid_callback_state, bid_callback_events) =
        bid_callback(bid_ctx, bid_callback_ctx, start_callback_state, bid);
    assert_eq!(bid_callback_events.len(), 0);
    // bid is added to claim map (bidder, currency: 0)
    assert_eq!(bid_callback_state.claim_map.len(), 1);
    let claim_map_entry = bid_callback_state.claim_map.get(&bidder);
    assert!(claim_map_entry.is_some());
    assert_eq!(
        *claim_map_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 99,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(
        bid_callback_state.highest_bidder,
        Bid {
            bidder: owner,
            amount: 0,
        }
    );
}

#[test]
pub fn test_bid_callback_not_highest_bid_cause_reserve() {
    let (mut init_state, _) = initialize_contract();
    init_state.reserve_price = 1000;
    init_state.min_increment = 100;
    assert_eq!(init_state.highest_bidder.amount, 0);
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let start_callback_ctx = create_callback_ctx(true);
    let (start_callback_state, _) = start_callback(start_ctx, start_callback_ctx, init_state);
    let bidder = get_bidder_address();
    let bid_ctx = create_ctx(bidder, 101);
    let bid_callback_ctx = create_callback_ctx(true);
    let bid = Bid {
        bidder,
        amount: 999,
    };
    assert_eq!(start_callback_state.claim_map.len(), 0);
    let (bid_callback_state, bid_callback_events) =
        bid_callback(bid_ctx, bid_callback_ctx, start_callback_state, bid);
    assert_eq!(bid_callback_events.len(), 0);
    // bid is added to claim map (bidder, currency: 0)
    assert_eq!(bid_callback_state.claim_map.len(), 1);
    let claim_map_entry = bid_callback_state.claim_map.get(&bidder);
    assert!(claim_map_entry.is_some());
    assert_eq!(
        *claim_map_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 999,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(
        bid_callback_state.highest_bidder,
        Bid {
            bidder: owner,
            amount: 0,
        }
    );
}

#[test]
#[should_panic]
pub fn test_bid_callback_transfer_unsuccessful() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let start_ctx = create_ctx(owner, 3);
    let start_callback_ctx = create_callback_ctx(true);
    let (start_callback_state, _) = start_callback(start_ctx, start_callback_ctx, init_state);
    let bidder = get_bidder_address();
    let bid_ctx = create_ctx(bidder, 4);
    let bid_callback_ctx = create_callback_ctx(false);
    let bid = Bid {
        bidder,
        amount: 1000,
    };
    bid_callback(bid_ctx, bid_callback_ctx, start_callback_state, bid);
}

#[test]
pub fn test_claim_no_entry() {
    let (mut init_state, _) = initialize_contract();
    let address = get_owner_address();
    init_state.add_to_claim_map(
        address,
        TokenClaim {
            tokens_for_bidding: 1000,
            tokens_for_sale: 0,
        },
    );
    let other_address = get_third_party_address();
    let claim_ctx = create_ctx(other_address, 4);
    let (claim_state, claim_events) = claim(claim_ctx, init_state);
    assert_eq!(claim_events.len(), 0);
    assert_eq!(claim_state.claim_map.len(), 1);
    let claim_entry = claim_state.claim_map.get(&address);
    assert!(claim_entry.is_some());
    assert_eq!(
        *claim_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 1000,
            tokens_for_sale: 0,
        }
    );
}

#[test]
pub fn test_claim_currency() {
    let (mut init_state, _) = initialize_contract();
    let address = get_owner_address();
    init_state.add_to_claim_map(
        address,
        TokenClaim {
            tokens_for_bidding: 1000,
            tokens_for_sale: 0,
        },
    );
    let claim_ctx = create_ctx(address, 4);
    let (claim_state, claim_events) = claim(claim_ctx, init_state.clone());
    assert_eq!(claim_state.claim_map.len(), 1);
    let claim_entry = claim_state.claim_map.get(&address);
    assert!(claim_entry.is_some());
    assert_eq!(
        *claim_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 0,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(claim_events.len(), 1);
    let event = claim_events.get(0).unwrap();
    let mut expected_event = EventGroup::builder();
    expected_event
        .call(get_currency_token_address(), Shortname::from_u32(1))
        .argument(get_owner_address())
        .argument(1000u128)
        .done();
    assert_eq!(*event, expected_event.build());
}

#[test]
pub fn test_claim_commodity() {
    let (mut init_state, _) = initialize_contract();
    let address = get_owner_address();
    init_state.add_to_claim_map(
        address,
        TokenClaim {
            tokens_for_bidding: 0,
            tokens_for_sale: 100,
        },
    );
    let claim_ctx = create_ctx(address, 4);
    let (claim_state, claim_events) = claim(claim_ctx, init_state.clone());
    assert_eq!(claim_state.claim_map.len(), 1);
    let claim_entry = claim_state.claim_map.get(&address);
    assert!(claim_entry.is_some());
    assert_eq!(
        *claim_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 0,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(claim_events.len(), 1);
    let event = claim_events.get(0).unwrap();
    let mut expected_event = EventGroup::builder();
    expected_event
        .call(get_commodity_token_address(), Shortname::from_u32(1))
        .argument(get_owner_address())
        .argument(100u128)
        .done();
    assert_eq!(*event, expected_event.build());
}

#[test]
pub fn test_claim_both() {
    let (mut init_state, _) = initialize_contract();
    let address = get_owner_address();
    init_state.add_to_claim_map(
        address,
        TokenClaim {
            tokens_for_bidding: 1000,
            tokens_for_sale: 100,
        },
    );
    let claim_ctx = create_ctx(address, 4);
    let (claim_state, claim_events) = claim(claim_ctx, init_state.clone());
    assert_eq!(claim_state.claim_map.len(), 1);
    let claim_entry = claim_state.claim_map.get(&address);
    assert!(claim_entry.is_some());
    assert_eq!(
        *claim_entry.unwrap(),
        TokenClaim {
            tokens_for_bidding: 0,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(claim_events.len(), 1);
    let event = claim_events.get(0).unwrap();
    let mut expected_event = EventGroup::builder();
    expected_event
        .call(get_currency_token_address(), Shortname::from_u32(1))
        .argument(get_owner_address())
        .argument(1000u128)
        .done();
    expected_event
        .call(get_commodity_token_address(), Shortname::from_u32(1))
        .argument(get_owner_address())
        .argument(100u128)
        .done();
    assert_eq!(*event, expected_event.build());
}

#[test]
pub fn test_execute() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let (started_state, _) =
        start_callback(create_ctx(owner, 3), create_callback_ctx(true), init_state);
    let bidder = get_bidder_address();
    let bid = Bid {
        bidder,
        amount: 2000,
    };
    let (bid_state, _) = bid_callback(
        create_ctx(bidder, 5),
        create_callback_ctx(true),
        started_state,
        bid,
    );
    // anyone can execute
    let third_party = get_third_party_address();
    // need block time >=102 since this is end time
    let ctx = create_ctx(third_party, 102);
    let (execute_state, execute_events) = execute(ctx, bid_state);
    assert_eq!(execute_events.len(), 0);
    assert_eq!(execute_state.status, ENDED);
    // both owner and bidder should have valid claims
    assert_eq!(execute_state.claim_map.len(), 2);
    let owner_claim = execute_state.claim_map.get(&owner);
    let bidder_claim = execute_state.claim_map.get(&bidder);
    assert!(owner_claim.is_some());
    assert!(bidder_claim.is_some());
    assert_eq!(
        *bidder_claim.unwrap(),
        TokenClaim {
            tokens_for_bidding: 0,
            tokens_for_sale: 100_000,
        }
    );
    assert_eq!(
        *owner_claim.unwrap(),
        TokenClaim {
            tokens_for_bidding: 2000,
            tokens_for_sale: 0,
        }
    );
}

#[test]
#[should_panic]
pub fn test_execute_early() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let (started_state, _) =
        start_callback(create_ctx(owner, 3), create_callback_ctx(true), init_state);
    let bidder = get_bidder_address();
    let bid = Bid {
        bidder,
        amount: 2000,
    };
    let (bid_state, _) = bid_callback(
        create_ctx(bidder, 5),
        create_callback_ctx(true),
        started_state,
        bid,
    );
    // anyone can execute
    let third_party = get_third_party_address();
    // need block time >=102 since this is end time
    let ctx = create_ctx(third_party, 101);
    execute(ctx, bid_state);
}

#[test]
#[should_panic]
pub fn test_execute_wrong_status() {
    let (init_state, _) = initialize_contract();
    // anyone can execute
    let third_party = get_third_party_address();
    // need block time >=102 since this is end time
    let ctx = create_ctx(third_party, 102);
    execute(ctx, init_state);
}

#[test]
pub fn test_cancel() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let (started_state, _) =
        start_callback(create_ctx(owner, 3), create_callback_ctx(true), init_state);
    let bidder = get_bidder_address();
    let bid = Bid {
        bidder,
        amount: 2000,
    };
    let (bid_state, _) = bid_callback(
        create_ctx(bidder, 5),
        create_callback_ctx(true),
        started_state,
        bid,
    );
    // need block time <102 since this is end time
    let ctx = create_ctx(owner, 101);
    let (cancel_state, cancel_events) = cancel(ctx, bid_state);
    assert_eq!(cancel_events.len(), 0);
    assert_eq!(cancel_state.status, CANCELLED);
    // both owner and bidder should have valid claims
    assert_eq!(cancel_state.claim_map.len(), 2);
    let owner_claim = cancel_state.claim_map.get(&owner);
    let bidder_claim = cancel_state.claim_map.get(&bidder);
    assert!(owner_claim.is_some());
    assert!(bidder_claim.is_some());
    assert_eq!(
        *bidder_claim.unwrap(),
        TokenClaim {
            tokens_for_bidding: 2000,
            tokens_for_sale: 0,
        }
    );
    assert_eq!(
        *owner_claim.unwrap(),
        TokenClaim {
            tokens_for_bidding: 0,
            tokens_for_sale: 100_000,
        }
    );
}

#[test]
#[should_panic]
pub fn test_cancel_not_owner() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let (started_state, _) =
        start_callback(create_ctx(owner, 3), create_callback_ctx(true), init_state);
    let bidder = get_bidder_address();
    let bid = Bid {
        bidder,
        amount: 2000,
    };
    let (bid_state, _) = bid_callback(
        create_ctx(bidder, 5),
        create_callback_ctx(true),
        started_state,
        bid,
    );
    // need block time <102 since this is end time
    let ctx = create_ctx(bidder, 101);
    cancel(ctx, bid_state);
}

#[test]
#[should_panic]
pub fn test_cancel_after_end_time() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let (started_state, _) =
        start_callback(create_ctx(owner, 3), create_callback_ctx(true), init_state);
    let bidder = get_bidder_address();
    let bid = Bid {
        bidder,
        amount: 2000,
    };
    let (bid_state, _) = bid_callback(
        create_ctx(bidder, 5),
        create_callback_ctx(true),
        started_state,
        bid,
    );
    // need block time <102 since this is end time
    let ctx = create_ctx(owner, 102);
    cancel(ctx, bid_state);
}

#[test]
#[should_panic]
pub fn test_cancel_not_bidding() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    // need block time <102 since this is end time
    let ctx = create_ctx(owner, 101);
    cancel(ctx, init_state);
}

#[test]
#[should_panic]
pub fn test_cancel_after_execute() {
    let (init_state, _) = initialize_contract();
    let owner = get_owner_address();
    let (started_state, _) =
        start_callback(create_ctx(owner, 3), create_callback_ctx(true), init_state);
    let bidder = get_bidder_address();
    let bid = Bid {
        bidder,
        amount: 2000,
    };
    let (bid_state, _) = bid_callback(
        create_ctx(bidder, 5),
        create_callback_ctx(true),
        started_state,
        bid,
    );
    // anyone can execute
    let third_party = get_third_party_address();
    // need block time >=102 since this is end time
    let ctx = create_ctx(third_party, 102);
    let (execute_state, execute_events) = execute(ctx, bid_state);
    let cancel_ctx = create_ctx(owner, 103);
    cancel(cancel_ctx, execute_state);
}
