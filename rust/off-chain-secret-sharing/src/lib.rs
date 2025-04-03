#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

mod signatures;

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use crate::signatures::{create_address, recover_public_key};
use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::off_chain::{
    HttpRequestData, HttpResponseData, OffChainContext, OffChainStorage, Transaction,
};
use pbc_traits::WriteRPC;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Node configuration
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
pub struct NodeConfig {
    /// Blockchain address of the engine. Used by contract to verify transactions.
    address: Address,
    /// HTTP endpoint of the engine. Used by users to find endpoint to interact with engines by.
    endpoint: String,
}

/// Identifier for a [`Sharing`].
type SharingId = u128;

/// Identifier of an engine.
type NodeIndex = usize;

/// Active secret sharing
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
struct Sharing {
    /// Owner of the secret sharing.
    owner: Address,
    /// Which nodes that have indicated completion of upload.
    nodes_with_completed_upload: Vec<bool>,
}

impl Sharing {
    /// Checks whether the authentication required for accessing the [`Sharing`].
    ///
    /// The authentication consists of a ECDSA signature over the secp256k1 curve signed by
    /// the owner of the sharing. The signature is placed in the Authorization header as a
    /// hex encoded string prefixed with `secp256k1 `.
    ///
    /// The message for checking the signature consists of the following all serialized as rpc.
    ///
    /// - Execution engine address
    /// - Contract address
    /// - Request method ("GET" or "PUT")
    /// - Request Uri ("/shares/{sharingId}")
    /// - Request body
    fn is_authenticated(
        &self,
        request: &HttpRequestData,
        off_chain_context: &OffChainContext,
    ) -> bool {
        let Some(header) = request.get_header_value("Authorization") else {
            return false;
        };
        let Some(token) = header.strip_prefix("secp256k1 ") else {
            return false;
        };
        let message: Vec<u8> = create_signature_message(request, off_chain_context);

        let Some(public_key) = recover_public_key(&message, token) else {
            return false;
        };

        let recovered_address = create_address(&public_key);

        recovered_address == self.owner
    }
}

/// State of the contract.
#[state]
pub struct ContractState {
    /// Node configurations
    nodes: Vec<NodeConfig>,
    /// Active secret sharings
    secret_sharings: AvlTreeMap<SharingId, Sharing>,
}

impl ContractState {
    fn node_index(&self, addr: &Address) -> Option<NodeIndex> {
        for engine_index in 0..self.nodes.len() {
            let address = self.nodes.get(engine_index).map(|c| c.address);
            match address {
                Some(address) if &address == addr => {
                    return Some(engine_index);
                }
                _ => {}
            }
        }
        None
    }
}

/// Initialize contract with the given node configurations.
///
/// ## RPC Arguments
///
/// - `nodes`: Configurations for all nodes that serve the contract.
#[init]
pub fn initialize(_ctx: ContractContext, nodes: Vec<NodeConfig>) -> ContractState {
    ContractState {
        nodes,
        secret_sharings: AvlTreeMap::new(),
    }
}

/// Register a new sharing with the given id.
///
/// User must then afterwards upload their sharing to each node.
///
/// ## RPC Arguments
///
/// - `sharing_id`: Identifier of the sharing. Must be unique wrt. all other existing sharings.
#[action(shortname = 0x01)]
pub fn register_sharing(
    ctx: ContractContext,
    mut state: ContractState,
    sharing_id: SharingId,
) -> ContractState {
    assert!(
        state.secret_sharings.get(&sharing_id).is_none(),
        "Cannot register sharing with the same identifier"
    );

    let nodes_with_completed_upload = vec![false; state.nodes.len()];

    state.secret_sharings.insert(
        sharing_id,
        Sharing {
            owner: ctx.sender,
            nodes_with_completed_upload,
        },
    );

    state
}

/// Register that the sharing with the given id has been completed for the calling node.
///
/// ## RPC Arguments
///
/// - `sharing_id`: Identifier of the sharing.
#[action(shortname = 0x02)]
pub fn register_shared(
    ctx: ContractContext,
    mut state: ContractState,
    sharing_id: SharingId,
) -> ContractState {
    let node_index = state
        .node_index(&ctx.sender)
        .expect("Caller is not one of the engines");

    let mut sharing = state
        .secret_sharings
        .get(&sharing_id)
        .expect("Unknown sharing");
    sharing.nodes_with_completed_upload[node_index] = true;

    state.secret_sharings.insert(sharing_id, sharing);

    state
}

const BUCKET_KEY_SHARES: [u8; 6] = *b"SHARES";

const JSON_RESPONSE_UNKNOWN_METHOD: &str = "{ \"error\": \"Invalid URL or method\" }";
const JSON_RESPONSE_UNKNOWN_SHARING: &str = "{ \"error\": \"Unknown sharing\" }";
const JSON_RESPONSE_UNAUTHORIZED: &str = "{ \"error\": \"Unauthorized\" }";
const JSON_RESPONSE_ALREADY_STORED: &str = "{ \"error\": \"Already stored\" }";
const JSON_RESPONSE_NOT_STORED: &str = "{ \"error\": \"Sharing haven't been stored yet\" }";

/// Off-chain receives an HTTP request.
///
/// This can either be a request for storing or loading a sharing.
///
/// ## RPC Arguments
///
/// - `request`: HTTP request to the sharing node.
///
/// ## Endpoints
///
/// ### Upload share
///
/// Path: `PUT /shares/<ID>`
///
/// Arguments:
/// - Path `ID`: Identifier of the sharing.
/// - Body: Sharing to upload as binary data.
/// - Authentication required.
///
/// Returns: Status code
///
/// Upload new sharing to the given id.
///
/// ### Download Share
///
/// Path: `GET /shares/<ID>`
///
/// Arguments:
/// - Path `ID`: Identifier of the sharing.
/// - Authentication required.
///
/// Returns: Status code
///
/// Download an existing sharing with the given id.
#[off_chain_on_http_request]
pub fn http_dispatch(
    mut ctx: OffChainContext,
    state: ContractState,
    request: HttpRequestData,
) -> HttpResponseData {
    let Some(http_action) = parse_action(&request) else {
        return HttpResponseData::new_with_str(404, JSON_RESPONSE_UNKNOWN_METHOD);
    };
    let sharing_id = http_action.sharing_id();

    let Some(sharing) = state.secret_sharings.get(&sharing_id) else {
        return HttpResponseData::new_with_str(404, JSON_RESPONSE_UNKNOWN_SHARING);
    };

    if !sharing.is_authenticated(&request, &ctx) {
        return HttpResponseData::new_with_str(401, JSON_RESPONSE_UNAUTHORIZED);
    }

    let mut storage: OffChainStorage<SharingId, Vec<u8>> = ctx.storage(&BUCKET_KEY_SHARES);
    let existing_data: Option<Vec<u8>> = storage.get(&sharing_id);

    match http_action {
        HttpAction::Store { .. } => {
            if existing_data.is_some() {
                return HttpResponseData::new_with_str(400, JSON_RESPONSE_ALREADY_STORED);
            }
            storage.insert(sharing_id, request.body);
            ctx.send_transaction(Transaction {
                address: ctx.contract_address,
                gas_cost: 1200,
                payload: {
                    let mut payload = vec![0x02];
                    sharing_id.rpc_write_to(&mut payload).unwrap();
                    payload
                },
            });
            HttpResponseData::new_with_str(201, "")
        }
        HttpAction::Load { .. } => {
            if let Some(data) = existing_data {
                HttpResponseData::new(200, data)
            } else {
                HttpResponseData::new_with_str(404, JSON_RESPONSE_NOT_STORED)
            }
        }
    }
}

/// Information about a specific HTTP action.
#[derive(Debug, Eq, PartialEq)]
pub enum HttpAction {
    /// Store new sharing.
    Store {
        /// Identifier of the sharing to store.
        sharing_id: SharingId,
    },
    /// Load existing sharing
    Load {
        /// Identifier of the sharing to load.
        sharing_id: SharingId,
    },
}

impl HttpAction {
    /// Identifier of the sharing in question.
    fn sharing_id(&self) -> SharingId {
        match self {
            Self::Store { sharing_id } => *sharing_id,
            Self::Load { sharing_id } => *sharing_id,
        }
    }
}

/// Parses an [`HttpAction`] from the given [`HttpRequestData`].
pub fn parse_action(request: &HttpRequestData) -> Option<HttpAction> {
    let split: Vec<&str> = request.uri.split("/").collect();
    if split.len() != 3 {
        return None;
    }
    let sharing_id = split[2].parse::<SharingId>().ok()?;
    if !split[0].is_empty() || split[1] != "shares" {
        return None;
    }

    match request.method.as_str().to_lowercase().as_ref() {
        "get" => Some(HttpAction::Load { sharing_id }),
        "put" => Some(HttpAction::Store { sharing_id }),
        _ => None,
    }
}

/// Create the message used for checking the signature. The message consists of the following
/// all serialized as rpc.
///
/// - Execution engine address
/// - Contract address
/// - Request method ("GET" or "PUT")
/// - Request Uri ("/shares/{sharingId}")
/// - Request body
pub fn create_signature_message(
    request: &HttpRequestData,
    off_chain_context: &OffChainContext,
) -> Vec<u8> {
    let mut message: Vec<u8> = vec![];
    off_chain_context
        .execution_engine_address
        .rpc_write_to(&mut message)
        .unwrap();
    off_chain_context
        .contract_address
        .rpc_write_to(&mut message)
        .unwrap();
    request.method.rpc_write_to(&mut message).unwrap();
    request.uri.rpc_write_to(&mut message).unwrap();
    request.body.rpc_write_to(&mut message).unwrap();

    message
}
