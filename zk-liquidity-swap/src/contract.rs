#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

mod pairwise_token_balances;
#[cfg(test)]
mod tests;
mod token_contract;

#[macro_use]
extern crate pbc_contract_codegen;
extern crate core;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::{Address, AddressType};
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::shortname::ShortnameZkComputation;
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_traits::ReadWriteState;
use read_write_state_derive::ReadWriteState;
use std::collections::VecDeque;
use std::mem::size_of;
use token_contract::TokenContract;

use pairwise_token_balances::{Balance, PairwiseTokenBalances, Token, TokenAmount};

const ZK_COMPUTE: ShortnameZkComputation = ShortnameZkComputation::from_u32(0x61);

/**
 * Metadata information associated with each individual variable.
 */
#[derive(ReadWriteState, Debug)]
pub struct SecretVarMetadata {
    /// Used to distinguish between input variables and output variables.
    is_output_variable: bool,
    /// If an swap input variable is marked with this, it means that the swap should only be
    /// performed if the swap is the first element of the worklist queue.
    only_if_at_front: bool,
}

/// This is the state of the contract which is persisted on the chain.
///
/// The #\[state\] macro generates serialization logic for the struct.
#[derive(Debug, Clone)]
#[state]
pub struct ContractState {
    /// The owner of the contract.
    pub contract_owner: Address,
    /// Address to use as the token pool entry in balances.
    pub token_pool_address: Address,
    /// The invariant used to calculate exchange rates.
    /// It's based on the 'constant product formula': x * y = k, k being the swap_constant.
    pub swap_constant: u128,
    /// Whether the contract is operable or not. PairwiseTokenBalances can still be withdraw when closed.
    pub is_closed: bool,
    /// User balances.
    pub balances: PairwiseTokenBalances,
    /// Worklist queue containing swaps that have yet to be performed.
    pub worklist: VecDeque<WorklistEntry>,
    /// Unused variables that should be removed during the next swap. Usually
    pub unused_variables: Vec<SecretVarId>,
}

/// An entry in the worklist, including the id of the variable containing the swap information, and
/// the address of the sender of the variable.
#[derive(Debug, ReadWriteState, CreateTypeSpec, Clone, PartialEq, Eq)]
pub struct WorklistEntry {
    /// Variable containing the swap amount and direction.
    variable_id: SecretVarId,
    /// Who sent the swap.
    sender: Address,
}

impl ContractState {
    /// Retrieves a copy of the pool that matches `token`.
    fn get_pools(&self) -> &Balance {
        self.balances.get_balance(&self.token_pool_address)
    }

    /// Checks for common invariants.
    fn assert_invariants(&self) {
        let pools = self.get_pools();
        assert!(pools.for_token(Token::A) * pools.for_token(Token::B) >= self.swap_constant);
    }
}

/// Initialize the contract.
///
/// ### Parameters
///
///   * `token_a_address`: The address of token A.
///
///   * `token_b_address`: The address of token B.
///
/// ### Returns
///
/// The new state object of type [`ContractState`] with all address fields initialized to their final state and remaining fields initialized to a default value.
///
#[init(zk = true)]
pub fn initialize(
    context: ContractContext,
    zk_state: ZkState<SecretVarMetadata>,
    token_a_address: Address,
    token_b_address: Address,
) -> (ContractState, Vec<EventGroup>) {
    assert_eq!(
        token_a_address.address_type,
        AddressType::PublicContract,
        "Tried to provide a non-Public Contract token for token A"
    );
    assert_eq!(
        token_b_address.address_type,
        AddressType::PublicContract,
        "Tried to provide a non-Public Contract token for token B"
    );
    assert_ne!(
        token_a_address, token_b_address,
        "Cannot initialize swap with duplicate tokens"
    );

    assert!(
        !cfg!(feature = "perform_calls"),
        "Callbacks not yet supported by ZK contracts"
    );

    let token_pool_address = context.contract_address;
    let mut balances = PairwiseTokenBalances::new(token_a_address, token_b_address);
    balances.deposit_to_user_balance(token_pool_address, Token::A, 0);

    let new_state = ContractState {
        token_pool_address,
        contract_owner: context.sender,
        swap_constant: 0,
        balances,
        is_closed: true,
        worklist: VecDeque::new(),
        unused_variables: Vec::new(),
    };

    (new_state, vec![])
}

/// Initialize pool {a, b} of the contract.
/// This can only be done by the contract owner and the contract has to be in its closed state.
///
/// ### Parameters:
///
///  * `token_address`: The address of the token {a, b}.
///
///  * `pool_size`: The desired size of token pool {a, b}.
///
/// # Returns
/// The unchanged state object of type [`ContractState`].
#[action(shortname = 0x10, zk = true)]
pub fn provide_liquidity(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    token_address: Address,
    pool_size: TokenAmount,
) -> (ContractState, Vec<EventGroup>) {
    assert_eq!(
        context.sender, state.contract_owner,
        "Only the contract owner can initialize contract pools"
    );
    assert!(
        state.is_closed,
        "Can only provide liquidity when the contract is closed"
    );

    let (to_token, _) = state
        .balances
        .deduce_from_to_tokens(&token_address)
        .expect("Provided unknown token address");

    state
        .balances
        .transfer_from_to(
            &context.sender,
            state.token_pool_address,
            to_token,
            pool_size,
        )
        .unwrap();

    // Check if both pools has been initialized. If so, open the contract and set the contract constant.
    let pool_balance = state.get_pools();
    if pool_balance.for_token(Token::A) > 0 && pool_balance.for_token(Token::B) > 0 {
        let swap_constant = pool_balance
            .for_token(Token::A)
            .checked_mul(pool_balance.for_token(Token::B));
        if let Some(swap_constant) = swap_constant {
            state.swap_constant = swap_constant;
            state.is_closed = false;
        }
    }

    (state, vec![])
}

/// Deposit token A or B into the calling users balance on the contract.
/// If the contract is closed, the action fails.
///
/// ### Parameters:
///
///  * `token_address`: The address of the deposited token contract.
///
///  * `amount`: The amount to deposit.
///
/// # Returns
/// The unchanged state object of type [`ContractState`].
#[action(shortname = 0x11, zk = true)]
pub fn deposit(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    token_address: Address,
    amount: TokenAmount,
) -> (ContractState, Vec<EventGroup>) {
    assert!(
        !state.is_closed || context.sender == state.contract_owner,
        "The contract is closed"
    );

    let (from_token, _) = state
        .balances
        .deduce_from_to_tokens(&token_address)
        .expect("Provided unknown token address");
    if cfg!(feature = "perform_calls") {
        let mut event_group_builder = EventGroup::builder();

        TokenContract::at_address(token_address).transfer_from(
            &mut event_group_builder,
            &context.sender,
            &state.token_pool_address,
            amount,
        );

        event_group_builder
            .with_callback(SHORTNAME_DEPOSIT_CALLBACK)
            .argument(from_token)
            .argument(amount)
            .done();

        (state, vec![event_group_builder.build()])
    } else {
        deposit_internal(context, state, from_token, amount)
    }
}

fn deposit_internal(
    context: ContractContext,
    mut state: ContractState,
    token: Token,
    amount: TokenAmount,
) -> (ContractState, Vec<EventGroup>) {
    state
        .balances
        .deposit_to_user_balance(context.sender, token, amount);
    (state, vec![])
}

/// Handles callback from `deposit`.
/// If the transfer event is successful the caller of `deposit` is added to the `state.balances`
/// adding `amount` to the `token` pool balance.
///
/// ### Parameters:
///
/// * `token`: Indicating the token pool balance of which to add `amount` to.
///
/// * `amount`: The desired amount to add to `token_type` pool balance.
///
/// ### Returns
///
/// The updated state object of type [`ContractState`] with an updated entry for the caller of `deposit`.
#[callback(shortname = 0x02, zk = true)]
pub fn deposit_callback(
    context: ContractContext,
    callback_context: CallbackContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    token: Token,
    amount: TokenAmount,
) -> (ContractState, Vec<EventGroup>) {
    assert!(callback_context.success, "Transfer did not succeed");
    deposit_internal(context, state, token, amount)
}

/// Swap `amount` of token A or B to the opposite token at the exchange rate dictated by `the constant product formula`.
/// The swap is executed on the user balances of tokens for the calling user.
/// If the contract is closed or if the caller does not have a sufficient balance of the token, the action fails.
///
/// ### Parameters:
///
///  * `only_if_at_front`: If true, the swap will only be performed if the swap variable
///  is the first in the worklist queue. This feature can be used to prevent frontrunning between
///  the time when this invocation is called, and when the swap variable is fully input.
///
///  * `amount` (ZK): The amount to swap of the token matching `input_token`.
///
/// # Returns
/// The updated state object of type [`ContractState`] yielding the result of the swap.
#[zk_on_secret_input(shortname = 0x13)]
pub fn swap(
    context: ContractContext,
    state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    only_if_at_front: bool,
) -> (
    ContractState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarMetadata>,
) {
    assert!(!state.is_closed, "The contract is closed");

    // Check that sender have non-zero balances. The swap would fail anyway if they don't have
    // anything to swap with.
    let balance = state.balances.get_balance(&context.sender);
    assert!(
        !balance.is_empty(),
        "Balances are both zero; nothing to swap with."
    );

    let input_def = ZkInputDef {
        seal: false,
        metadata: SecretVarMetadata {
            only_if_at_front,
            is_output_variable: false,
        },
        expected_bit_lengths: vec![AmountAndDirection::BITS],
    };
    (state, vec![], input_def)
}

fn start_next_in_queue(state: ContractState) -> (ContractState, Vec<ZkStateChange>) {
    let next_id = match state.worklist.front() {
        None => return (state, vec![]),
        Some(next_id) => next_id.variable_id,
    };

    let output_metadata = SecretVarMetadata {
        only_if_at_front: false,
        is_output_variable: true,
    };

    (
        state,
        vec![ZkStateChange::start_computation_with_inputs(
            ZK_COMPUTE,
            vec![output_metadata],
            vec![next_id],
        )],
    )
}

/// Automatic callback for when some previously announced user variable is fully input.
///
/// Will create a new worklist entry, and possibly start computation, if no previous computation is
/// active.
#[zk_on_variable_inputted]
pub fn swap_variable_inputted(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    variable_id: SecretVarId,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let only_if_at_front = zk_state
        .get_variable(variable_id)
        .unwrap()
        .metadata
        .only_if_at_front;

    let worklist_entry = WorklistEntry {
        variable_id,
        sender: context.sender,
    };

    // Swap already in progress; let's wait for it to finish
    if !state.worklist.is_empty() {
        if only_if_at_front {
            state.unused_variables.push(worklist_entry.variable_id)
        } else {
            state.worklist.push_back(worklist_entry);
        }
        (state, vec![], vec![])
    } else {
        // Swap not in progress, push and start
        state.worklist.push_back(worklist_entry);
        let (state, zk_events) = start_next_in_queue(state);
        (state, vec![], zk_events)
    }
}

/// Will immediately open the result of the computation.
#[zk_on_compute_complete]
pub fn computation_complete(
    _context: ContractContext,
    state: ContractState,
    _zk_state: ZkState<SecretVarMetadata>,
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

/// Called when the swap result is available. The invocation will perform the swap and start the
/// next swap in the worklist, if any are present. Any unused variables will be removed here.
#[zk_on_variables_opened]
pub fn swap_opened(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    opened_result_variables: Vec<SecretVarId>,
) -> (ContractState, Vec<EventGroup>, Vec<ZkStateChange>) {
    // Invariant checking
    assert_eq!(
        opened_result_variables.len(),
        1,
        "Should only be possible to open current swap variable"
    );

    let worklist_entry_processed = state.worklist.pop_front().unwrap();

    // Determine
    let amount_and_direction: AmountAndDirection =
        read_amount_and_direction(&zk_state, opened_result_variables.get(0).unwrap()).unwrap();
    let (token_from, token_to) = state
        .balances
        .deduce_from_to_tokens_b(amount_and_direction.is_from_a);

    let mut variables_to_delete = state.unused_variables;
    state.unused_variables = Vec::new();

    let new_state_result = perform_swap(
        &state,
        worklist_entry_processed.sender,
        token_from,
        token_to,
        amount_and_direction.amount,
    );

    let state = match new_state_result {
        Ok(state) => state,
        Err(_) => state,
    };

    // Determine what to do afterwards
    variables_to_delete.push(worklist_entry_processed.variable_id);
    variables_to_delete.extend(opened_result_variables);
    let (state, mut zk_events) = start_next_in_queue(state);
    zk_events.insert(
        0,
        ZkStateChange::OutputComplete {
            variables_to_delete,
        },
    );

    // Delete old variables
    (state, vec![], zk_events)
}

/// Computes how many `token_to` tokens should be given for the having swapped in the given amount
/// of `token_from` tokens.
fn calculate_token_to_amount(
    state: &ContractState,
    token_from: Token,
    token_to: Token,
    token_from_sent_amount: TokenSwapAmount,
) -> Result<TokenSwapAmount, &str> {
    let pool_balance = state.get_pools();
    let from_pool_value = pool_balance.for_token(token_from);
    let to_pool_value = pool_balance.for_token(token_to);

    let new_from_pool_value = from_pool_value
        .checked_add(token_from_sent_amount)
        .ok_or("Overflow in token pool")?;
    let new_to_pool_value = u128_division_ceil(state.swap_constant, new_from_pool_value)?;

    to_pool_value
        .checked_sub(new_to_pool_value)
        .ok_or("Underflow in token pool")
}

fn perform_swap(
    state_original: &ContractState,
    sender: Address,
    token_from: Token,
    token_to: Token,
    token_from_sent_amount: TokenSwapAmount,
) -> Result<ContractState, String> {
    let token_to_revc_amount =
        calculate_token_to_amount(state_original, token_from, token_to, token_from_sent_amount)?;

    let mut state = state_original.clone();
    state.balances.transfer_from_to(
        &sender,
        state.token_pool_address,
        token_from,
        token_from_sent_amount,
    )?;
    state.balances.transfer_from_to(
        &state.token_pool_address,
        sender,
        token_to,
        token_to_revc_amount,
    )?;

    state.assert_invariants();

    Ok(state)
}

/// Withdraw `amount` of token A or B from the contract for the calling user.
/// This fails if `amount` is larger than the user balance of the corresponding token.
///
/// It preemptively updates the state of the user's balance before making the transfer.
/// This means that if the transfer fails, the contract could end up with more money than it has registered, which is acceptable.
/// This is to incentivize the user to spend enough gas to complete the transfer.
///
/// ### Parameters:
///
///  * `token_address`: The address of the token contract to withdraw to.
///
///  * `amount`: The amount to withdraw.
///
/// # Returns
/// The unchanged state object of type [`ContractState`].
#[action(shortname = 0x14, zk = true)]
pub fn withdraw(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
    token_address: Address,
    amount: TokenAmount,
) -> (ContractState, Vec<EventGroup>) {
    let (token_from, _) = state
        .balances
        .deduce_from_to_tokens(&token_address)
        .expect("Provided unknown token address");

    state
        .balances
        .withdraw_from_user_balance(&context.sender, token_from, amount)
        .unwrap();

    if cfg!(feature = "perform_calls") {
        let mut event_group_builder = EventGroup::builder();

        TokenContract::at_address(token_address).transfer(
            &mut event_group_builder,
            &context.sender,
            amount,
        );

        (state, vec![event_group_builder.build()])
    } else {
        (state, vec![])
    }
}

/// Empties the pools into the contract owner's balance and closes the contract.
/// Fails if called by anyone but the contract owner.
///
/// ### Returns
///
/// The updated state object of type [`ContractState`].
#[action(shortname = 0x15, zk = true)]
pub fn close_pools(
    context: ContractContext,
    mut state: ContractState,
    zk_state: ZkState<SecretVarMetadata>,
) -> (ContractState, Vec<EventGroup>) {
    assert_eq!(
        context.sender, state.contract_owner,
        "Only the contract owner can close the pools"
    );
    assert!(!state.is_closed, "The contract is closed");

    let pool_balance = state.get_pools().clone();
    state
        .balances
        .transfer_from_to(
            &state.token_pool_address,
            state.contract_owner,
            Token::A,
            pool_balance.for_token(Token::A),
        )
        .unwrap();
    state
        .balances
        .transfer_from_to(
            &state.token_pool_address,
            state.contract_owner,
            Token::B,
            pool_balance.for_token(Token::B),
        )
        .unwrap();

    // Close contract
    state.is_closed = true;

    // Assert correctly closed
    let pool_balance = state.get_pools();
    assert_eq!(pool_balance.for_token(Token::A), 0);
    assert_eq!(pool_balance.for_token(Token::B), 0);

    (state, vec![])
}

/// * HELPER FUNCTIONS *

/// Divides two [`u128`] types and rounds up.
///
/// ### Parameters:
///
/// * `numerator`: The numerator for the division.
///
/// * `denominator`: The denominator for the division.
///
/// ### Returns:
///
/// The result of the division, rounded up, of type [`u128`].
fn u128_division_ceil(numerator: u128, denominator: u128) -> Result<u128, &'static str> {
    let div_floor = numerator
        .checked_div(denominator)
        .ok_or("Division by zero")?;
    let rem = numerator
        .checked_rem(denominator)
        .ok_or("Division by zero")?;
    Ok(div_floor + u128::from(rem != 0))
}

/// Type used for swap amounts.
///
/// Currently a much smaller type than `TokenAmount`, due to limitations in zk-computations.
pub type TokenSwapAmount = u128;

/// Public version of `AmountAndDirection`.
#[derive(ReadWriteState)]
#[repr(C)]
pub struct AmountAndDirection {
    /// Amount of tokens to swap
    pub amount: TokenSwapAmount,
    /// Whether to swap from or to a.
    pub is_from_a: bool,
}

impl AmountAndDirection {
    /// Number of bits used for [`AmountAndDirection`]
    const BITS: u32 = TokenSwapAmount::BITS + 8 * size_of::<bool>() as u32;
}

fn read_amount_and_direction(
    zk_state: &ZkState<SecretVarMetadata>,
    variable_id: &SecretVarId,
) -> Option<AmountAndDirection> {
    read_variable_data(zk_state, variable_id)
}

/// Reads variable data for `variable_id` as `T`.
fn read_variable_data<T: ReadWriteState>(
    zk_state: &ZkState<SecretVarMetadata>,
    variable_id: &SecretVarId,
) -> Option<T> {
    let variable = zk_state.get_variable(*variable_id)?;
    let buffer = variable.data.as_ref()?;
    Some(T::state_read_from(&mut buffer.as_slice()))
}
