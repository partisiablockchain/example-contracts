#![doc = include_str!("../README.md")]
#![allow(unused_variables)]
// Allow for the warning in the README.
#![allow(rustdoc::broken_intra_doc_links)]

mod http_router;
mod signatures;

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use crate::http_router::HttpMethod::{Get, Put};
use crate::http_router::HttpRouter;
use crate::signatures::{create_address, recover_public_key};
use create_type_spec_derive::CreateTypeSpec;
use matchit::Params;
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::off_chain::{
    HttpRequestData, HttpResponseData, OffChainContext, OffChainStorage,
};
use pbc_contract_common::Hash;
use pbc_traits::WriteRPC;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;
use std::io::{Read, Write};

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
    /// SHA256 Commitment to specific shares per engine. Prevents an engine from corrupting the
    /// share without the receipient's knowledge.
    share_commitments: Vec<Hash>,
    /// Which nodes that have indicated completion of upload.
    nodes_with_completed_upload: Vec<bool>,
}

/// Individual secret-share; one part of a [`Sharing`].
#[derive(ReadWriteState)]
struct SecretShare {
    /// A nonce used to prevent brute force attacks of small secrets.
    ///
    /// [Rainbow table](https://en.wikipedia.org/wiki/Rainbow_table)-like attacks would be possible
    /// if this field weren't present, due to the possiblity of an attacker computing the
    /// commitments that different shares would hash to. If an attacker uncovers the share of
    /// enough [`Sharing::share_commitments`] it is quite possible for the attacker to determine
    /// the underlying plaintext.
    ///
    /// This field helps to prevent this by enforcing that all shares start with 32 bytes of
    /// data.
    nonce: [u8; 32],
    /// The underlying secret share.
    secret_share: Vec<u8>,
}

impl SecretShare {
    /// Get [`Hash`] of the [`SecretShare`]. This includes both the actual secret-sharing data, and
    /// the nonce.
    fn hash(&self) -> Hash {
        Hash::digest(&self.write_to_vec())
    }

    /// Serialize [`SecretShare`] to a vec.
    ///
    /// Inverse of [`SecretShare::read_from`].
    fn write_to_vec(&self) -> Vec<u8> {
        let mut serialized = vec![];
        serialized.write_all(&self.nonce).unwrap();
        serialized.write_all(&self.secret_share).unwrap();
        serialized
    }

    /// Read [`SecretShare`] from a reader (such as a byte-stream).
    ///
    /// Inverse of [`SecretShare::write_to_vec`].
    ///
    /// Format:
    /// - nonce: 32 bytes
    /// - secret_share: remaining bytes (not size-prefixed)
    fn read_from<R: Read>(mut reader: R) -> Result<Self, std::io::Error> {
        let mut nonce = [0; 32];
        reader.read_exact(&mut nonce)?;
        let mut secret_share = vec![];
        reader.read_to_end(&mut secret_share)?;
        Ok(SecretShare {
            nonce,
            secret_share,
        })
    }
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

    /// Asserts that the http request is authenticated for this sharing.
    ///
    /// Returns 401 Error if the request is not authenticated
    fn assert_is_authenticated(
        &self,
        request: &HttpRequestData,
        off_chain_context: &OffChainContext,
    ) -> Result<(), HttpResponseData> {
        validate_condition_or_produce_http_error(
            self.is_authenticated(request, off_chain_context),
            401,
            JSON_RESPONSE_UNAUTHORIZED,
        )
    }
}

/// Utility method for either returning an ok status, or returning a request error, if the
/// condition returns false.
///
/// ## Arguments
///
/// - `cond`: The realized condition value.
/// - `err_status`: The error status if the condition fails.
/// - `response_body`: The response body if the condition fails.
///
/// ## Returns
///
/// Empty `Ok` if the condition succeeds, or an `Err` with the given error status and response if
/// the condition fails.
fn validate_condition_or_produce_http_error(
    cond: bool,
    err_status: u32,
    response_body: &'static str,
) -> Result<(), HttpResponseData> {
    if cond {
        Ok(())
    } else {
        Err(HttpResponseData::new_with_str(err_status, response_body))
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

    /// Get the sharing of for a specific sharing id.
    ///
    /// Returns 404 Error if the sharing doesn't exist in the state
    fn get_sharing(&self, sharing_id: SharingId) -> Result<Sharing, HttpResponseData> {
        self.secret_sharings
            .get(&sharing_id)
            .ok_or(HttpResponseData::new_with_str(
                404,
                JSON_RESPONSE_UNKNOWN_SHARING,
            ))
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
/// - `share_commitments`: Commitment for each share.
#[action(shortname = 0x01)]
pub fn register_sharing(
    ctx: ContractContext,
    mut state: ContractState,
    sharing_id: SharingId,
    share_commitments: Vec<Hash>,
) -> ContractState {
    assert!(
        state.secret_sharings.get(&sharing_id).is_none(),
        "Cannot register sharing with the same identifier"
    );
    assert_eq!(
        share_commitments.len(),
        state.nodes.len(),
        "Invalid number of share commitments"
    );

    let nodes_with_completed_upload = vec![false; state.nodes.len()];

    state.secret_sharings.insert(
        sharing_id,
        Sharing {
            owner: ctx.sender,
            share_commitments,
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

const JSON_RESPONSE_UNKNOWN_URL: &str = "{ \"error\": \"Invalid URL\" }";
const JSON_RESPONSE_MALFORMED: &str = "{ \"error\": \"Malformed request\" }";
const JSON_RESPONSE_UNKNOWN_METHOD: &str = "{ \"error\": \"Invalid method\" }";
const JSON_RESPONSE_UNKNOWN_SHARING: &str = "{ \"error\": \"Unknown sharing\" }";
const JSON_RESPONSE_UNAUTHORIZED: &str = "{ \"error\": \"Unauthorized\" }";
const JSON_RESPONSE_ALREADY_STORED: &str = "{ \"error\": \"Already stored\" }";
const JSON_RESPONSE_NOT_STORED: &str = "{ \"error\": \"Sharing haven't been stored yet\" }";
const JSON_RESPONSE_COMMITMENT_MISMATCH: &str =
    "{ \"error\": \"User uploaded data doesn't match commitment\" }";

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
    ctx: OffChainContext,
    state: ContractState,
    request: HttpRequestData,
) -> HttpResponseData {
    let mut router: HttpRouter = HttpRouter::new();
    router.insert("/shares/{id}", Get(http_sharing_get));
    router.insert("/shares/{id}", Put(http_sharing_put));

    let result = router.dispatch(ctx, state, request);
    result.unwrap_or_else(|err| err)
}

/// Upload new sharing to the given id.
///
/// Path: `PUT /shares/<ID>`
///
/// Arguments:
/// - Path `ID`: Identifier of the sharing.
/// - Body: Sharing to upload as binary data.
/// - Authentication required.
///
/// Returns: Status code
fn http_sharing_put(
    mut ctx: OffChainContext,
    state: ContractState,
    request: HttpRequestData,
    params: Params,
) -> Result<HttpResponseData, HttpResponseData> {
    let sharing_id = parse_sharing_id(params)?;
    let sharing = state.get_sharing(sharing_id)?;
    sharing.assert_is_authenticated(&request, &ctx)?;

    let node_index = state.node_index(&ctx.execution_engine_address).unwrap();

    let Ok(secret_share) = SecretShare::read_from(&mut request.body.as_slice()) else {
        return Err(HttpResponseData::new_with_str(400, JSON_RESPONSE_MALFORMED));
    };

    let expected_hash_of_share = sharing.share_commitments.get(node_index).unwrap();
    validate_condition_or_produce_http_error(
        &secret_share.hash() == expected_hash_of_share,
        401,
        JSON_RESPONSE_COMMITMENT_MISMATCH,
    )?;

    let mut storage = secret_share_storage(&mut ctx);
    let existing_data: Option<SecretShare> = storage.get(&sharing_id);

    validate_condition_or_produce_http_error(
        existing_data.is_none(),
        409,
        JSON_RESPONSE_ALREADY_STORED,
    )?;

    storage.insert(sharing_id, secret_share);
    ctx.send_transaction_to_contract(
        {
            let mut payload = vec![0x02];
            sharing_id.rpc_write_to(&mut payload).unwrap();
            payload
        },
        1200,
    );
    Ok(HttpResponseData::new_with_str(201, ""))
}

/// Download an existing sharing with the given id.
///
/// Path: `GET /shares/<ID>`
///
/// Arguments:
/// - Path `ID`: Identifier of the sharing.
/// - Authentication required.
///
/// Returns: Status code
fn http_sharing_get(
    mut ctx: OffChainContext,
    state: ContractState,
    request: HttpRequestData,
    params: Params,
) -> Result<HttpResponseData, HttpResponseData> {
    let sharing_id = parse_sharing_id(params)?;
    let sharing = state.get_sharing(sharing_id)?;
    sharing.assert_is_authenticated(&request, &ctx)?;

    let existing_data: Option<SecretShare> = secret_share_storage(&mut ctx).get(&sharing_id);
    if let Some(data) = existing_data {
        Ok(HttpResponseData::new(200, data.write_to_vec()))
    } else {
        Err(HttpResponseData::new_with_str(
            404,
            JSON_RESPONSE_NOT_STORED,
        ))
    }
}

fn secret_share_storage(ctx: &mut OffChainContext) -> OffChainStorage<SharingId, SecretShare> {
    ctx.storage(&BUCKET_KEY_SHARES)
}

/// Parse a sharing id from the params given in the request url
fn parse_sharing_id(params: Params) -> Result<SharingId, HttpResponseData> {
    params
        .get("id")
        .unwrap()
        .parse()
        .map_err(|_| HttpResponseData::new_with_str(400, JSON_RESPONSE_MALFORMED))
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
