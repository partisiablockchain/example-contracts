#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

mod zk_compute;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::zk::{AttestationId, SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_traits::ReadWriteState;
use pbc_zk::Sbu32;
use read_write_rpc_derive::ReadRPC;
use read_write_rpc_derive::WriteRPC;
use read_write_state_derive::ReadWriteState;

/// Secret variable metadata. Contains unique ID of the bidder.
#[derive(ReadWriteState, ReadRPC, WriteRPC, Debug)]
struct SecretVarMetadata {
    is_bid: bool,
}

/// Number of bids required before starting auction computation.
const MIN_NUM_BIDDERS: u32 = 3;

/// Type of tracking bid amount
type BidAmountPublic = u32;

/// Tracks whether a user have placed their bid or not.
#[derive(ReadWriteState, ReadRPC, WriteRPC, Debug, CreateTypeSpec)]
struct RegisteredBidder {
    /// External id of the bidder. Part of the attestation.
    external_id: ExternalId,
    /// Tracks whether a user have placed their bid or not.
    have_already_bid: bool,
}

/// An id that is assigned in the [`register_bidder`] invocation. Part of the attestation,
/// allowing external systems to easily use their own idenfiers.
///
/// Part of the attested data when an auction is won.
#[derive(ReadWriteState, ReadRPC, WriteRPC, CreateTypeSpec, Debug)]
struct ExternalId {
    /// Identifier bytes
    id_bytes: Vec<u8>,
}

/// Struct used for [`register_bidders`]. Includes both the bidder's PBC blockchain [`Address`],
/// and any external id that the owner has decided to attach.
///
/// Part of the attested data when an auction is won.
#[derive(ReadWriteState, ReadRPC, WriteRPC, CreateTypeSpec, Debug)]
struct AddressAndExternalId {
    address: Address,
    external_id: ExternalId,
}

/// This state of the contract.
#[state]
struct ContractState {
    /// Owner of the contract
    owner: Address,
    /// Registered bidders - only registered bidders are allowed to bid.
    registered_bidders: AvlTreeMap<Address, RegisteredBidder>,
    /// Whether the auction has already begun?
    auction_begun: bool,
    /// The auction result
    auction_result: Option<AuctionResult>,
}

#[derive(ReadWriteState, CreateTypeSpec, ReadRPC)]
struct AuctionResult {
    /// Address of the auction winner
    winner: AddressAndExternalId,
    /// The winning bid
    second_highest_bid: BidAmountPublic,
}

/// Initializes contract
///
/// Note that owner is set to whoever initializes the contact.
#[init(zk = true)]
fn initialize(context: ContractContext, zk_state: ZkState<SecretVarMetadata>) -> ContractState {
    ContractState {
        owner: context.sender,
        registered_bidders: AvlTreeMap::new(),
        auction_begun: false,
        auction_result: None,
    }
}

/// Registers new bidders, by specifying their [`Address`]es and their [`ExternalId`].
///
/// [`ExternalId`] is useful for layer 2 solutions, where the contract acts as a secondary system;
/// the ids can be set to anything that might be needed in the primary system. [`ExternalId`]s are
/// not strictly needed, and can be left empty if they are unneeded.
///
/// Multiple bidders can be registered at once.
///
/// Requirements:
///
/// - Only the sender can add bidders.
/// - The auction must not already have been started (by calling [`start_auction`].)
/// - Bidders must not already be registered.
#[action(shortname = 0x30, zk = true)]
fn register_bidders(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    bidder_definitions: Vec<AddressAndExternalId>,
) -> ContractState {
    assert!(
        !state.auction_begun,
        "Cannot register bidders after auction has begun"
    );
    assert_eq!(
        context.sender, state.owner,
        "Only the owner can register bidders"
    );

    for bidder_def in bidder_definitions {
        assert!(
            !state.registered_bidders.contains_key(&bidder_def.address),
            "Duplicate bidder address: {:?}",
            bidder_def.address
        );

        state.registered_bidders.insert(
            bidder_def.address,
            RegisteredBidder {
                external_id: bidder_def.external_id,
                have_already_bid: false,
            },
        );
    }

    state
}

/// Adds another bid variable to the ZkState.
///
/// Requirements:
///
/// - Only the bidders can place bids.
/// - The auction must not already have been started (by calling [`start_auction`].)
/// - Bidders must not already have placed a bid.
#[zk_on_secret_input(shortname = 0x40)]
fn place_bid(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata, Sbu32>,
) {
    assert!(
        !state.auction_begun,
        "Cannot place bid after auction has begun"
    );

    // Only bidders that have not already placed bids can bid.
    let Some(mut bidder_info) = state.registered_bidders.get(&context.sender) else {
        panic!("{:?} is not a registered bidder", context.sender)
    };
    assert!(
        !bidder_info.have_already_bid,
        "Each bidder is only allowed to place one bid: {:?}",
        context.sender,
    );

    let input_def = ZkInputDef::with_metadata(None, SecretVarMetadata { is_bid: true });

    // Update state to track the bid.
    bidder_info.have_already_bid = true;
    state.registered_bidders.insert(context.sender, bidder_info);

    (state, vec![], input_def)
}

/// Singleton to indicate that a [`SecretVarMetadata`] is a result, and not a bid.
const NOT_A_BID: SecretVarMetadata = SecretVarMetadata { is_bid: false };

/// Starts the auction computation, which determines the winner of the auction among the existing
/// bids.
///
/// Requirements:
/// - Can only be run by the owner.
/// - The auction must not already have started.
/// - And at least [`MIN_NUM_BIDDERS`] must have placed their bids.
///
/// The second price auction computation is beyond this call, involving several ZK computation steps.
#[action(shortname = 0x01, zk = true)]
fn start_auction(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert!(
        !state.auction_begun,
        "Cannot start auction after it has already begun"
    );
    assert_eq!(
        context.sender, state.owner,
        "Only contract owner can start the auction"
    );
    let amount_of_bidders = zk_state.secret_variables.len() as u32;
    assert!(
        amount_of_bidders >= MIN_NUM_BIDDERS,
        "At least {MIN_NUM_BIDDERS} bidders must have submitted bids for the auction to start",
    );

    state.auction_begun = true;

    (
        state,
        vec![],
        vec![zk_compute::run_auction::start(
            Some(close_auction::SHORTNAME),
            [&NOT_A_BID, &NOT_A_BID],
        )],
    )
}

/// Automatically called when the computation is completed
///
/// The only thing we do is instantly open/declassify the output variables.
#[zk_on_compute_complete(shortname = 0x42)]
fn close_auction(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    output_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    (
        state,
        vec![],
        vec![ZkStateChange::OpenVariables {
            variables: output_variables,
        }],
    )
}

/// Automatically called when the auction result is declassified. Updates state to contain result,
/// and requests attestation from nodes.
#[zk_on_variables_opened]
fn open_auction_variable(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    opened_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let highest_bid_id: SecretVarId = read_variable(&zk_state, opened_variables.first()).unwrap();

    let winner_bid = zk_state
        .get_variable(highest_bid_id)
        .expect("Variable must exist");

    let highest_bidder = state.registered_bidders.get(&winner_bid.owner).unwrap();

    let auction_result = AuctionResult {
        winner: AddressAndExternalId {
            external_id: highest_bidder.external_id,
            address: winner_bid.owner,
        },
        second_highest_bid: read_variable(&zk_state, opened_variables.get(1)).unwrap(),
    };

    let attest_request = ZkStateChange::Attest {
        data_to_attest: serialize_as_state(&auction_result),
    };

    (state, vec![], vec![attest_request])
}

/// Automatically called when some data is attested
#[zk_on_attestation_complete]
fn auction_results_attested(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    attestation_id: AttestationId,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let attestation = zk_state.get_attestation(attestation_id).unwrap();

    assert_eq!(attestation.signatures.len(), 4, "Must have four signatures");

    assert!(
        attestation.signatures.iter().all(|sig| sig.is_some()),
        "Attestation must be complete"
    );

    let auction_result = AuctionResult::state_read_from(&mut attestation.data.as_slice());

    state.auction_result = Some(auction_result);

    (state, vec![], vec![ZkStateChange::ContractDone])
}

/// Writes some value as RPC data.
fn serialize_as_state<T: ReadWriteState>(it: &T) -> Vec<u8> {
    let mut output: Vec<u8> = vec![];
    it.state_write_to(&mut output).expect("Could not serialize");
    output
}

/// Reads a variable's data as some state value
fn read_variable<T: ReadWriteState>(
    zk_state: &ZkState<SecretVarMetadata>,
    variable_id: Option<&SecretVarId>,
) -> Option<T> {
    zk_state.get_variable(*variable_id?)?.open_value::<T>()
}
