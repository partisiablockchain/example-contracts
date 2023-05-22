//! This is an example Conditional Escrow Transfer contract
//!
//! Conditional Escrow Transfer allows a sender to put tokens into an escrow contract which a
//! receiver can receive when a condition has been fulfilled.
//! The escrow transfer contract handles a specific token type.
//! A sender can place tokens into escrow specifying the receiver and an approver that signals
//! condition fulfilment and a deadline.
//! The approver can signal fulfilment of the condition. The condition itself is not part of the
//! contract, only the signalling of the fulfilment of the condition.
//! The receiver can claim the tokens when the condition has been fulfilled.
//! The sender can claim the tokens when the deadline is met and the condition is not fulfilled.

#[macro_use]
extern crate pbc_contract_codegen;

use pbc_contract_common::address::{Address, AddressType, Shortname};
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;

/// Constants for different phases of the contract.

/// Initial state after contract creation.
const STATE_CREATED: u8 = 0;
/// State after tokens have been transferred to the contract.
/// The contract now awaits approval from the approver.
const STATE_AWAITING_APPROVAL: u8 = 1;
/// State after the approver has signalled fulfilment of the condition
const STATE_APPROVED: u8 = 2;

/// The contract state.
///
/// ### Fields:
///
///   * `sender`: [`Address`], the sender of the tokens
///
///   * `receiver`: [`Address`], the receiver of tokens following approval of the condition.
///
///   * `approver`: [`Address`], the approver that can signal fulfilment of the condition.
///
///   * `token_type`: [`Address`], the address of the token used in the contract.
///
///   * `balance`: [`u128`], the amount of tokens currently in the contract.
///
///   * `start_time_millis`: [`i64`], the start time of the contract milliseconds.
///
///   * `end_time_millis`: [`i64`], the dead line of the contract in milliseconds.
///
///   * `status`: [`u8`], the current status of the contract.
///
#[state]
pub struct ContractState {
    sender: Address,
    receiver: Address,
    approver: Address,
    token_type: Address,
    balance: u128,
    start_time_millis: i64,
    end_time_millis: i64,
    status: u8,
}

/// Initial function to bootstrap the contract's state.
///
/// ### Parameters
///
///   * `context`: [`ContractContext`] - the contract context containing sender and chain information.
///
///   * `receiver`: [`Address`] - the receiver of tokens following approval of the condition.
///
///   * `approver`: [`Address`], the approver that can signal fulfilment of the condition.
///
///   * `token_type`: [`Address`], the address of the token used in the contract.
///
///   * `hours_until_deadline`: [`u32`], the number of hours until the deadline gets passed.
///
/// ### Returns
///
/// The new state object of type [`ContractState`] with the initial state being `STATE_CREATED`.
///
#[init]
pub fn initialize(
    context: ContractContext,
    sender: Address,
    receiver: Address,
    approver: Address,
    token_type: Address,
    hours_until_deadline: u32,
) -> ContractState {
    if token_type.address_type != AddressType::PublicContract {
        panic!("Tried to create a contract selling a non publicContract token");
    }
    let millis_until_deadline = i64::from(hours_until_deadline) * 60 * 60 * 1000;
    let end_time_millis = context.block_production_time + millis_until_deadline;
    ContractState {
        sender,
        receiver,
        approver,
        token_type,
        balance: 0,
        start_time_millis: context.block_production_time,
        end_time_millis,
        status: STATE_CREATED,
    }
}

/// Action for the sender to deposit tokens into the contract.
/// Throws an error if not called by the `sender` or if
/// the status is not `STATE_CREATED`.
/// The function creates a transfer event of tokens from the `sender` to the contract, and
/// a callback to `deposit_callback`.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`ContractState`], the current state of the contract.
///
/// * `amount`: [`u128`], the amount of tokens to deposit
///
/// ### Returns
///
/// The unchanged state object of type [`ContractState`] and the event group containing the
/// transfer event and the callback event.
///
#[action(shortname = 0x01)]
pub fn deposit(
    context: ContractContext,
    state: ContractState,
    amount: u128,
) -> (ContractState, Vec<EventGroup>) {
    if context.sender != state.sender {
        panic!("Deposit can only be called by the sender");
    }
    if state.status == STATE_APPROVED {
        panic!("Cannot deposit tokens after the condition has been fulfilled");
    }
    if context.block_production_time > state.end_time_millis {
        panic!("Cannot deposit tokens after deadline is passed");
    }
    // Create transfer event of tokens from the sender to the contract
    // transfer should callback to deposit_callback
    let mut e = EventGroup::builder();
    e.call(state.token_type, token_contract_transfer_from())
        .argument(context.sender)
        .argument(context.contract_address)
        .argument(amount)
        .done();
    e.with_callback(SHORTNAME_DEPOSIT_CALLBACK)
        .argument(amount)
        .done();
    let event_group: EventGroup = e.build();

    (state, vec![event_group])
}

/// Callback for depositing tokens. If the transfer was successful the status of the contract
/// is updated to `STATE_AWAITING_APPROVAL`. Otherwise the callback panics.
///
/// ### Parameters:
///
/// * `ctx`: [`ContractContext`], the contractContext for the callback.
///
/// * `callback_ctx`: [`CallbackContext`], the callbackContext.
///
/// * `state`: [`ContractState`], the current state of the contract.
///
/// ### Returns
///
/// The new state object of type [`ContractState`].
///
#[callback(shortname = 0x02)]
pub fn deposit_callback(
    _ctx: ContractContext,
    callback_ctx: CallbackContext,
    state: ContractState,
    amount: u128,
) -> (ContractState, Vec<EventGroup>) {
    if !callback_ctx.success {
        panic!("Transfer event did not succeed for deposit");
    }
    let mut new_state = state;
    new_state.balance += amount;
    new_state.status = STATE_AWAITING_APPROVAL;
    (new_state, vec![])
}

/// Action for signalling fulfilment of the condition. Panics if the deadline of the
/// contract has been passed, if the caller is not the correct `approver` or if the contract is
/// not in state `STATE_AWAITING_APPROVAL`. Otherwise updates the status of the contract to `STATE_APPROVED`.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the contractContext for the action.
///
/// * `state`: [`ContractState`], the current state of the contract.
///
/// ### Returns
///
/// The new state object of type [`ContractState`].
///
#[action(shortname = 0x03)]
pub fn approve(context: ContractContext, state: ContractState) -> (ContractState, Vec<EventGroup>) {
    if context.sender != state.approver {
        panic!("Only the designated approver can approve")
    }
    if context.block_production_time > state.end_time_millis {
        panic!("Condition was fulfilled after deadline was passed");
    }
    if state.status != STATE_AWAITING_APPROVAL {
        panic!("Tried to approve when status was not STATE_AWAITING_APPROVAL")
    }

    let mut new_state = state;
    new_state.status = STATE_APPROVED;
    (new_state, vec![])
}

/// Action for claiming tokens.
/// The `receiver` is allowed to claim the tokens if the status is `STATE_APPROVED`.
/// The `sender` is allowed to claim the tokens if the status is `AWAITING_APPROVAL`
/// and the deadline has been passed.
/// No other addresses can claim tokens
/// If the tokens are claimed a corresponding transfer event is created and the status is
/// updated to `TOKENS_CLAIMED`.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`], the context for the action call.
///
/// * `state`: [`ContractState`], the current state of the contract.
///
/// ### Returns
///
/// The new state object of type [`ContractState`] and an event group possibly containing a
/// transfer event.
///
#[action(shortname = 0x04)]
pub fn claim(context: ContractContext, state: ContractState) -> (ContractState, Vec<EventGroup>) {
    let can_claim = context.sender == state.receiver || context.sender == state.sender;
    if !can_claim {
        panic!("Only the sender and the receiver in the escrow transfer can claim tokens");
    }
    if state.status == STATE_CREATED {
        panic!("Cannot claim tokens when no tokens have been deposited");
    }
    if state.balance == 0 {
        panic!("Cannot claim tokens when balance is zero");
    }
    if context.sender == state.receiver && state.status != STATE_APPROVED {
        panic!("The receiver cannot claim unless transfer condition has been fulfilled");
    }
    if context.sender == state.sender {
        if state.status == STATE_APPROVED {
            panic!("The sender cannot claim tokens since the condition has been fulfilled");
        }
        if context.block_production_time < state.end_time_millis {
            panic!("The sender cannot claim tokens before the deadline is passed");
        }
    }

    let mut e = EventGroup::builder();
    e.call(state.token_type, token_contract_transfer())
        .argument(context.sender)
        .argument(state.balance)
        .done();
    let event_group = e.build();

    let mut new_state = state;
    new_state.balance = 0;

    (new_state, vec![event_group])
}

/// Token contract actions
#[inline]
fn token_contract_transfer() -> Shortname {
    Shortname::from_u32(0x01)
}

#[inline]
fn token_contract_transfer_from() -> Shortname {
    Shortname::from_u32(0x03)
}
