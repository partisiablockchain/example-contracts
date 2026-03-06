#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeSet;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::off_chain::{HttpRequestData, HttpResponseData, OffChainContext};
use pbc_contract_common::Hash;
use pbc_traits::ReadRPC;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

// Error messages
const NOT_POST_REQUEST: &str = "Not a POST request";
const NO_AUTHORIZATION_HEADER: &str = "No \"Authorization\" header provided";
const BAD_AUTHORIZATION_HEADER_FIELD: &str =
    "The \"Authorization\" header field must be formatted as \"Authorization: Bearer <TOKEN>\"";
const INVALID_TOKEN: &str = "Token is unauthorized";

/// Request from a user to proxy a transaction to a target smart contract.
///
/// Contains the transaction details and gas cost needed by the off-chain
/// execution engine to forward the request.
#[derive(Clone, ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
pub struct TransactionRequest {
    /// The target smart contract address to forward the payload to.
    target_smart_contract: Address,
    /// The serialized transaction to be forwarded.
    payload: Vec<u8>,
    /// The total gas needed for the transaction, in gas units.
    gas: u64,
}

/// Transaction payload to be forwarded to a target smart contract.
///
/// This is sent on-chain by the execution engine after authorizing
/// the user's request.
#[derive(Clone, ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
pub struct TransactionPayload {
    /// The target smart contract address to forward the payload to.
    target_smart_contract: Address,
    /// The serialized transaction to be forwarded.
    payload: Vec<u8>,
}

/// Engine configuration
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
pub struct EngineConfig {
    /// Blockchain address of the engine. Used by contract to verify transactions.
    address: Address,
    /// HTTP endpoint of the engine. Used by users to find endpoint to interact with engines by.
    endpoint: String,
}

/// State of the contract.
#[state]
pub struct ProxyState {
    /// The hashes of valid authorization tokens.
    pub token_hashes: AvlTreeSet<Hash>,
    /// Execution engines allocated to this contract
    pub engines: Vec<EngineConfig>,
}

impl ProxyState {
    /// Checks if the hash of the given token exists in `token_hashes`
    ///
    /// ### Parameters
    ///
    /// * `token`: the token whose hash is being checked for existence in `token_hashes`.
    ///
    /// ### Returns
    ///
    /// True if the token's hash exists in `token_hashes` and false otherwise.
    ///
    pub fn is_valid_token(&self, token: String) -> bool {
        self.token_hashes.contains(&Hash::digest(token))
    }

    /// Asserts that an address is that of an assigned execution engine.
    ///
    /// ### Parameters
    ///
    /// * `address`: the address to check for validity.
    ///
    /// ### Returns
    ///
    /// Nothing if the address is that of an assigned execution engine.
    /// Panics otherwise.
    ///
    pub fn assert_is_assigned_engine(&self, address: Address) {
        assert!(
            self.engines
                .iter()
                .any(|engine_config| engine_config.address == address),
            "The address {:?} is not a known execution engine address",
            address
        );
    }
}

/// Initialize a new proxy contract
///
/// ### Parameters
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `token_hashes_vec` - the hashes such that if a token's hash is in this list,
/// it is considered authorized.
/// * `engines` - the engine configs of execution engines allocated for this smart contract.
///
/// ### Returns
///
/// The initial state of the proxy contract.
///
#[init]
pub fn initialize(
    ctx: ContractContext,
    token_hashes_vec: Vec<Hash>,
    engines: Vec<EngineConfig>,
) -> ProxyState {
    let mut token_hashes: AvlTreeSet<Hash> = AvlTreeSet::new();
    token_hashes_vec
        .into_iter()
        .for_each(|token_hash| token_hashes.insert(token_hash));

    ProxyState {
        token_hashes,
        engines,
    }
}

/// Forwards a transaction payload to the target smart contract.
///
/// ### Parameters
///
/// * `ctx`: the context for the action call.
/// * `state`: the current state of the contract.
/// * `transaction_payload`: the transaction payload,
///     its target smart contract, and gas cost.
///
/// ### Returns
///
/// An unchanged contract state and an eventgroup that calls
/// the RPC payload at the requested target smart contract.
///
#[action(shortname = 0x01)]
pub fn forward(
    ctx: ContractContext,
    state: ProxyState,
    transaction_payload: TransactionPayload,
) -> (ProxyState, Vec<EventGroup>) {
    state.assert_is_assigned_engine(ctx.sender);

    // Create an event group that contains the forwarded transaction
    let mut event_group_builder = EventGroup::builder();
    event_group_builder
        .call_with_rpc(
            transaction_payload.target_smart_contract,
            transaction_payload.payload,
        )
        .done();
    let event_group = event_group_builder.build();

    (state, vec![event_group])
}

/// Offchain HTTP hook for clients to send POST payloads to.
///
/// The HTTP request must contain:
/// - An authorization header with the key "Authorization" and
///   a body of the format: "Bearer \<TOKEN\>".
/// - A request body that contains a serialized `TransactionPayload`
///   that represents the transaction the user wishes to proxy.
///
/// A request that is successfully authorized and is forwarded
/// to the on-chain component will be responded with a 200 code.
///
/// Ill-formatted authorization headers will be responded with a
/// 400 code and a body that describes the expected formatting.
///
/// Well-formatted authorization headers that fail to authorize
/// will be responded with a 401 code.
///
/// ### Parameters
///
/// * `off_chain_context`: the context for the HTTP call.
/// * `state`: the current state of the contract.
/// * `request`: the HTTP request.
///
/// ### Returns
///
/// An HTTP response.
///
#[off_chain_on_http_request]
pub fn http_dispatch(
    mut off_chain_context: OffChainContext,
    state: ProxyState,
    request: HttpRequestData,
) -> HttpResponseData {
    // Check if it is a `POST` request
    if request.method != "POST" {
        return HttpResponseData::new_with_str(400, NOT_POST_REQUEST);
    }

    // Extract `Authorization` header
    let Some(auth_header) = request.get_header_value("Authorization") else {
        return HttpResponseData::new_with_str(400, NO_AUTHORIZATION_HEADER);
    };

    // Extract the token from an `Authorization` header of the format: Bearer <TOKEN>
    let Some(token) = auth_header.strip_prefix("Bearer ") else {
        return HttpResponseData::new_with_str(400, BAD_AUTHORIZATION_HEADER_FIELD);
    };

    // Check if H(token) \in `token_hashes`
    if !state.is_valid_token(token.to_string()) {
        return HttpResponseData::new_with_str(401, INVALID_TOKEN);
    }

    // Deserialize `request.body`
    let transaction_request = TransactionRequest::rpc_read_from(&mut request.body.as_slice());

    // Forward the payload to on-chain
    off_chain_context.send_transaction_to_contract(
        forward::rpc(TransactionPayload {
            target_smart_contract: transaction_request.target_smart_contract,
            payload: transaction_request.payload,
        }),
        transaction_request.gas,
    );

    // Send update to user/client
    HttpResponseData::new_with_str(200, "Ok")
}
