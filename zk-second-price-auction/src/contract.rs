#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::shortname::ShortnameZkComputation;
use pbc_contract_common::zk::{
    AttestationId, CalculationStatus, SecretVarId, ZkInputDef, ZkState, ZkStateChange,
};
use pbc_traits::{ReadRPC, ReadWriteState, WriteRPC};
use read_write_rpc_derive::ReadRPC;
use read_write_rpc_derive::WriteRPC;
use read_write_state_derive::ReadWriteState;

/// Id of a contract bidder.
#[repr(transparent)]
#[derive(PartialEq, ReadRPC, WriteRPC, ReadWriteState, Debug, Clone, Copy, CreateTypeSpec)]
#[non_exhaustive]
struct BidderId {
    id: i32,
}

/// Secret variable metadata. Contains unique ID of the bidder.
#[derive(ReadWriteState, ReadRPC, WriteRPC, Debug)]
struct SecretVarMetadata {
    bidder_id: BidderId,
}

/// The size of the MPC bid input variables.
const BITLENGTH_OF_SECRET_BID_VARIABLES: [u32; 1] = [32];

/// Number of bids required before starting auction computation.
const MIN_NUM_BIDDERS: u32 = 3;

const ZK_COMPUTE: ShortnameZkComputation = ShortnameZkComputation::from_u32(0x61);

/// Type of tracking bid amount
type BidAmount = i32;

/// This state of the contract.
#[state]
struct ContractState {
    /// Owner of the contract
    owner: Address,
    /// Registered bidders - only registered bidders are allowed to bid.
    registered_bidders: Vec<RegisteredBidder>,
    /// The auction result
    auction_result: Option<AuctionResult>,
}

#[derive(Clone, ReadWriteState, CreateTypeSpec, ReadRPC, WriteRPC)]
struct AuctionResult {
    /// Bidder id of the auction winner
    winner: BidderId,
    /// The winning bid
    second_highest_bid: BidAmount,
}

/// Representation of a registered bidder with an address
#[derive(Clone, ReadWriteState, CreateTypeSpec)]
struct RegisteredBidder {
    bidder_id: BidderId,
    address: Address,
}

/// Initializes contract
///
/// Note that owner is set to whoever initializes the contact.
#[init(zk = true)]
fn initialize(context: ContractContext, zk_state: ZkState<SecretVarMetadata>) -> ContractState {
    ContractState {
        owner: context.sender,
        registered_bidders: Vec::new(),
        auction_result: None,
    }
}

/// Registers a bidder with an address and updates the state accordingly.
////
/// Ensures that only the owner of the contract is able to register bidders.
#[action(shortname = 0x30, zk = true)]
fn register_bidder(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    bidder_id: i32,
    address: Address,
) -> ContractState {
    let bidder_id = BidderId { id: bidder_id };

    assert_eq!(
        context.sender, state.owner,
        "Only the owner can register bidders"
    );

    assert!(
        state
            .registered_bidders
            .iter()
            .all(|x| x.address != address),
        "Duplicate bidder address: {address:?}",
    );

    assert!(
        state
            .registered_bidders
            .iter()
            .all(|x| x.bidder_id != bidder_id),
        "Duplicate bidder id: {bidder_id:?}",
    );

    state
        .registered_bidders
        .push(RegisteredBidder { bidder_id, address });

    state
}

/// Adds another bid variable to the ZkState.
///
/// The ZkInputDef encodes that variables should have size [`BITLENGTH_OF_SECRET_BID_VARIABLES`].
#[zk_on_secret_input(shortname = 0x40)]
fn add_bid(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata>,
) {
    let bidder_info = state
        .registered_bidders
        .iter()
        .find(|x| x.address == context.sender);

    let bidder_info = match bidder_info {
        Some(bidder_info) => bidder_info,
        None => panic!("{:?} is not a registered bidder", context.sender),
    };

    // Assert that only one bid is placed per bidder
    assert!(
        zk_state
            .secret_variables
            .iter()
            .chain(zk_state.pending_inputs.iter())
            .all(|v| v.owner != context.sender),
        "Each bidder is only allowed to send one bid. : {:?}",
        bidder_info.bidder_id,
    );

    let input_def = ZkInputDef {
        seal: false,
        metadata: SecretVarMetadata {
            bidder_id: bidder_info.bidder_id,
        },
        expected_bit_lengths: BITLENGTH_OF_SECRET_BID_VARIABLES.to_vec(),
    };

    (state, vec![], input_def)
}

/// Allows the owner of the contract to start the computation, computing the winner of the auction.
///
/// The second price auction computation is beyond this call, involving several ZK computation steps.
#[action(shortname = 0x01, zk = true)]
fn compute_winner(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        zk_state.calculation_state,
        CalculationStatus::Waiting,
        "Computation must start from Waiting state, but was {:?}",
        zk_state.calculation_state,
    );
    assert_eq!(
        zk_state.data_attestations.len(),
        0,
        "Auction must have exactly zero data_attestations at this point"
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

    (
        state,
        vec![],
        vec![ZkStateChange::start_computation(
            ZK_COMPUTE,
            vec![
                SecretVarMetadata {
                    bidder_id: BidderId { id: -1 },
                },
                SecretVarMetadata {
                    bidder_id: BidderId { id: -1 },
                },
            ],
        )],
    )
}

/// Automatically called when the computation is completed
///
/// The only thing we do is instantly open/declassify the output variables.
#[zk_on_compute_complete]
fn auction_compute_complete(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    output_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        zk_state.data_attestations.len(),
        0,
        "Auction must have exactly zero data_attestations at this point"
    );
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
    assert_eq!(
        opened_variables.len(),
        2,
        "Unexpected number of output variables"
    );
    assert_eq!(
        zk_state.data_attestations.len(),
        0,
        "Auction must have exactly zero data_attestations at this point"
    );

    let auction_result = AuctionResult {
        winner: read_variable(&zk_state, opened_variables.get(0)),
        second_highest_bid: read_variable(&zk_state, opened_variables.get(1)),
    };

    let attest_request = ZkStateChange::Attest {
        data_to_attest: serialize_as_big_endian(&auction_result),
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
    assert_eq!(
        zk_state.data_attestations.len(),
        1,
        "Auction must have exactly one attestation"
    );
    let attestation = zk_state.get_attestation(attestation_id).unwrap();

    assert_eq!(attestation.signatures.len(), 4, "Must have four signatures");

    let auction_result = AuctionResult::rpc_read_from(&mut attestation.data.as_slice());

    state.auction_result = Some(auction_result);

    (state, vec![], vec![ZkStateChange::ContractDone])
}

/// Writes some value as RPC data.
fn serialize_as_big_endian<T: WriteRPC>(it: &T) -> Vec<u8> {
    let mut output: Vec<u8> = vec![];
    it.rpc_write_to(&mut output).expect("Could not serialize");
    output
}

/// Reads a variable's data as some state value
fn read_variable<T: ReadWriteState>(
    zk_state: &ZkState<SecretVarMetadata>,
    variable_id: Option<&SecretVarId>,
) -> T {
    let variable_id = *variable_id.unwrap();
    let variable = zk_state.get_variable(variable_id).unwrap();
    let buffer: Vec<u8> = variable.data.clone().unwrap();
    T::state_read_from(&mut buffer.as_slice())
}
