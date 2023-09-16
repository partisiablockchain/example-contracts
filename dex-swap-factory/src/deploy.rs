//! Small utility library to provide contract deployment.
//!
//! Does not support Zero-knowledge contracts.

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::{Address, AddressType, Shortname};
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroupBuilder;
use read_write_state_derive::ReadWriteState;
use std::cmp::min;

/// Address of the public WASM deployment contract.
const ADDRESS_DEPLOY_PUBLIC: Address = Address {
    address_type: AddressType::SystemContract,
    identifier: [
        0x97, 0xa0, 0xe2, 0x38, 0xe9, 0x24, 0x02, 0x5b, 0xad, 0x14, 0x4a, 0xa0, 0xc4, 0x91, 0x3e,
        0x46, 0x30, 0x8f, 0x9a, 0x4d,
    ],
};

/// Contract version type. Does not have specific semantics with the exception of being
/// monotonically increasing.
pub type ContractVersion = u64;

/// [`Shortname`] for invoking public deployment on [`ADDRESS_DEPLOY_PUB`].
const SHORTNAME_DEPLOY_PUB: Shortname = Shortname::from_u32(0x01);

/// The magic bytes at the start of any given WASM file.
const WASM_MAGIC_BYTES: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

/// The magic bytes at the start of any given PBC ABI file.
const PBCABI_MAGIC_BYTES: [u8; 6] = [b'P', b'B', b'C', b'A', b'B', b'I'];

/// Deployment information for a contract.
#[derive(ReadWriteState, CreateTypeSpec)]
#[non_exhaustive]
pub struct DeployableContract {
    /// Byte code for contract.
    pub bytecode: Vec<u8>,
    /// Application binary interface for contract.
    pub abi: Vec<u8>,
    /// Version of contract.
    pub version: ContractVersion,
}

/// Extracts the a prefix from the given slice.
fn clone_prefix(slice: &[u8], wanted_length: usize) -> Vec<u8> {
    let len = min(wanted_length, slice.len());
    let mut out: Vec<u8> = vec![0; len];
    out.clone_from_slice(&slice[0..len]);
    out
}

impl DeployableContract {
    /// Creates new [`DeployableContract`] and validates it.
    pub fn new(bytecode: Vec<u8>, abi: Vec<u8>, version: ContractVersion) -> DeployableContract {
        let deployable_contract = DeployableContract {
            bytecode,
            abi,
            version,
        };
        deployable_contract.validate();
        deployable_contract
    }

    /// Performs basic validation on the [`DeployableContract`], ensuring that bytecode is
    /// WASM, and checking that ABI field contains ABI data.
    pub fn validate(&self) {
        assert!(
            self.bytecode.starts_with(&WASM_MAGIC_BYTES),
            "Bytecode does not contain WASM code: {:02X?}",
            clone_prefix(&self.bytecode, 10),
        );
        assert!(
            self.abi.starts_with(&PBCABI_MAGIC_BYTES),
            "ABI data invalid: {:02X?}",
            clone_prefix(&self.abi, 10),
        );
    }
}

/// Adds invocation for deploying a contract with some initializable data.
///
/// ### Parameters:
///
/// - `deploy_data`: Contract to deploy.
/// - `builder`: The event group builder to append deployment interaction to.
/// - `initialization_rpc`: RPC to initialize contract with.
/// - `ctx`: [`ContractContext`] of the contract. Used to determine the [`Address`] of the deployed contract.
///
/// ### Returns:
///
/// Returns the [`Address`] of the deployed contract.
pub(crate) fn deploy_contract(
    deploy_data: &DeployableContract,
    builder: &mut EventGroupBuilder,
    initialization_rpc: Vec<u8>,
    ctx: &ContractContext,
) -> Address {
    builder
        .call(ADDRESS_DEPLOY_PUBLIC, SHORTNAME_DEPLOY_PUB)
        .argument(deploy_data.bytecode.clone())
        .argument(deploy_data.abi.clone())
        .argument(initialization_rpc)
        .done();

    Address {
        address_type: AddressType::PublicContract,
        identifier: ctx.original_transaction.bytes[12..32].try_into().unwrap(),
    }
}
