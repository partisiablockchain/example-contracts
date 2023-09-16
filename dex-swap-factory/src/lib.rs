#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use pbc_traits::WriteRPC;
use read_write_rpc_derive::{ReadRPC, WriteRPC};
use read_write_state_derive::ReadWriteState;

mod deploy;
mod permission;

use permission::Permission;

/// Swap pair.
///
/// ## Normalized form
///
/// - [`TokenPair::token_a_address`] must be [`Ord`]ered  before [`TokenPair::token_b_address`].
///
/// This normalization can be accomplished by calling [`TokenPair::normalize`].
#[derive(ReadRPC, ReadWriteState, CreateTypeSpec)]
pub struct TokenPair {
    /// The first token address.
    pub token_a_address: Address,
    /// The second token address.
    pub token_b_address: Address,
}

impl TokenPair {
    /// Performs the normalization mentioned in [`TokenPair`] documentation.
    pub fn normalize(self) -> Self {
        let mut tokens = [self.token_a_address, self.token_b_address];
        tokens.sort();
        Self {
            token_a_address: tokens[0],
            token_b_address: tokens[1],
        }
    }
}

/// Information about a deployed swap contract.
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct SwapContractInfo {
    /// Version of the contract.
    pub contract_version: deploy::ContractVersion,
    /// The [`TokenPair`] covered by the contract.
    pub token_pair: TokenPair,
    /// Whether contract was successfully deployed.
    pub successfully_deployed: bool,
}

/// Contract state.
#[state]
pub struct SwapFactoryState {
    /// Permission indicating who are allowed to update the swap contract.
    pub permission_update_swap: Permission,
    /// Permission indicating who are allowed to deploy new swap contracts.
    pub permission_deploy_swap: Permission,
    /// Deployed swap contracts.
    pub swap_contracts: SortedVecMap<Address, SwapContractInfo>,
    /// Deployment information for new swap contract.
    pub swap_contract_binary: Option<deploy::DeployableContract>,
    /// Swap fee for new contracts.
    pub swap_fee_per_mille: u16,
}

/// Initial action to create the initial state.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], initial context.
/// * `swap_contract_wasm`: [`Vec<u8>`], wasm bytes of a swap contract.
/// * `swap_contract_abi`: [`Vec<u8>`], abi bytes of a swap contract.
///
/// ### Returns:
/// The initial state of type [`SwapFactoryState`].
#[init]
pub fn initialize(
    ctx: ContractContext,
    permission_update_swap: Permission,
    permission_deploy_swap: Permission,
    swap_fee_per_mille: u16,
) -> SwapFactoryState {
    SwapFactoryState {
        permission_update_swap,
        permission_deploy_swap,
        swap_contracts: SortedVecMap::new(),
        swap_contract_binary: None,
        swap_fee_per_mille,
    }
}

/// Action for replacing swap contract binary.
#[action(shortname = 0x10)]
pub fn update_swap_binary(
    ctx: ContractContext,
    mut state: SwapFactoryState,
    swap_contract_binary: Vec<u8>,
    swap_contract_abi: Vec<u8>,
    swap_contract_version: deploy::ContractVersion,
) -> SwapFactoryState {
    state
        .permission_update_swap
        .assert_permission_for(&ctx.sender, "update swap");
    if let Some(prev_binary) = state.swap_contract_binary {
        assert!(
            prev_binary.version < swap_contract_version,
            "Versions must be increasing: Previous version {} should be less than new version {}",
            prev_binary.version,
            swap_contract_version
        );
    }
    state.swap_contract_binary = Some(deploy::DeployableContract::new(
        swap_contract_binary,
        swap_contract_abi,
        swap_contract_version,
    ));
    state
}

/// Action to deploy a new swap contract with given [`TokenPair`].
///
/// The address of the new swap contract is computed from the original transaction hash. Only
/// people with [`SwapFactoryState::permission_update_swap`] can add new swap contracts. and the
/// [`TokenPair`] has to be unique.  This creates an event to the public deploy contract as well as
/// creates a callback to [`deploy_swap_contract_callback`].
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the action call.
/// * `state`: [`SwapFactoryState`], the state before the call.
/// * `token_pair`: [`TokenPair`], the [`TokenPair`] of the new swap contract.
///
/// ### Returns:
///
/// The new state of type [`SwapFactoryState`].
#[action(shortname = 0x01)]
pub fn deploy_swap_contract(
    ctx: ContractContext,
    mut state: SwapFactoryState,
    token_pair: TokenPair,
) -> (SwapFactoryState, Vec<EventGroup>) {
    let token_pair = token_pair.normalize();

    state
        .permission_deploy_swap
        .assert_permission_for(&ctx.sender, "deploy swap");

    let swap_contract_binary = state
        .swap_contract_binary
        .as_ref()
        .expect("Cannot deploy swap contract, when swap contract binary has not be set!");

    let mut event_group = EventGroup::builder();

    let init_msg = SwapContractInitMsg {
        token_a_address: token_pair.token_a_address,
        token_b_address: token_pair.token_b_address,
        swap_fee_per_mille: state.swap_fee_per_mille,
    };

    let contract_address = deploy::deploy_contract(
        swap_contract_binary,
        &mut event_group,
        init_msg.to_init_bytes(),
        &ctx,
    );

    state.swap_contracts.insert(
        contract_address,
        SwapContractInfo {
            token_pair,
            contract_version: swap_contract_binary.version,
            successfully_deployed: false,
        },
    );

    event_group
        .with_callback(SHORTNAME_DEPLOY_SWAP_CONTRACT_CALLBACK)
        .with_cost(1000)
        .argument(contract_address)
        .done();

    (state, vec![event_group.build()])
}

/// Callback for adding a new swap contract. Triggered from [`deploy_swap_contract`].
///
/// A ping invocation will be performed to check that the deployed contract was successfully
/// deployed; this call callbacks into [`swap_contract_exists_callback`].
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the call.
/// * `callback_ctx`: [`CallbackContext`], the context of the callback.
/// * `state`: [`SwapFactoryState`], the state before the call.
/// * `swap_address`: [`Address`], the address of the the new swap contract.
///
/// ### Returns:
/// The new state of type [`SwapFactoryState`].
#[callback(shortname = 0x01)]
pub fn deploy_swap_contract_callback(
    ctx: ContractContext,
    callback_ctx: CallbackContext,
    state: SwapFactoryState,
    swap_address: Address,
) -> (SwapFactoryState, Vec<EventGroup>) {
    let mut event_group = EventGroup::builder();

    event_group.ping(swap_address, None);
    event_group
        .with_callback(SHORTNAME_SWAP_CONTRACT_EXISTS_CALLBACK)
        .argument(swap_address)
        .done();

    (state, vec![event_group.build()])
}

/// Callback invoked to check whether a swap contract has been deployed successfully. Triggered
/// from [`deploy_swap_contract_callback`].
///
/// - If deployed correctly, it will have it's [`SwapContractInfo::successfully_deployed`] flag
///   enabled, marking that it is safe to use.
/// - If deployment fails, the contract's entry will be removed.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the context of the call.
/// * `callback_ctx`: [`CallbackContext`], the context of the callback.
/// * `state`: [`SwapFactoryState`], the state before the call.
/// * `swap_address`: [`Address`], the address of the the new swap contract.
///
/// ### Returns:
/// The new state of type [`SwapFactoryState`].
#[callback(shortname = 0x02)]
pub fn swap_contract_exists_callback(
    ctx: ContractContext,
    callback_ctx: CallbackContext,
    mut state: SwapFactoryState,
    swap_address: Address,
) -> SwapFactoryState {
    if !callback_ctx.results[0].succeeded {
        state.swap_contracts.remove(&swap_address);
    } else {
        state
            .swap_contracts
            .get_mut(&swap_address)
            .unwrap()
            .successfully_deployed = true;
    }
    state
}

/// Action for removing a contract from the [swap directory](SwapFactoryState::swap_contracts).
///
/// ### Parameters
///
/// - `address`: Address of the contracts to remove. Unknown contracts are ignored.
///
/// ### Returns:
///
/// The new state of type [`SwapFactoryState`].
#[action(shortname = 0x02)]
pub fn delist_swap_contract(
    ctx: ContractContext,
    mut state: SwapFactoryState,
    address: Address,
) -> SwapFactoryState {
    state.swap_contracts.remove(&address);
    state
}

/// Shortname for contract initialization.
const INIT_SHORTNAME: [u8; 5] = [0xff, 0xff, 0xff, 0xff, 0x0f];

/// RPC utility for initializing liquidity swap contracts.
#[derive(WriteRPC, CreateTypeSpec)]
pub struct SwapContractInitMsg {
    /// Address of the first token in the swap.
    pub token_a_address: Address,
    /// Address of the second token in the swap.
    pub token_b_address: Address,
    /// Fee for swapping, defined in per milles.
    pub swap_fee_per_mille: u16,
}

impl SwapContractInitMsg {
    /// Creates RPC from the init message.
    fn to_init_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];
        INIT_SHORTNAME.rpc_write_to(&mut bytes).unwrap();
        self.rpc_write_to(&mut bytes).unwrap();
        bytes
    }
}
