#![doc = include_str!("../README.md")]

mod interact_mpc20;
mod math;
#[cfg(test)]
mod tests;
mod token_balances;

#[macro_use]
extern crate pbc_contract_codegen;
extern crate core;

use pbc_contract_common::address::Address;
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;
use std::ops::RangeInclusive;

pub use token_balances::Token;
use token_balances::{TokenBalances, TokensInOut};

/// Token amounts type. Should be set to what the token contract uses.
pub type TokenAmount = u128;

/// The range of allowed [`LiquiditySwapContractState::swap_fee_per_mille`].
pub const ALLOWED_FEE_PER_MILLE: RangeInclusive<u16> = 0..=1000;

/// This is the state of the contract which is persisted on the chain.
///
/// The #\[state\] macro generates serialization logic for the struct.
#[state]
pub struct LiquiditySwapContractState {
    /// The address of this contract
    pub liquidity_pool_address: Address,
    /// The fee for making swaps per mille. Must be in range [`ALLOWED_FEE_PER_MILLE`].
    pub swap_fee_per_mille: u16,
    /// The map containing all token balances of all users and the contract itself. <br>
    /// The contract should always have a balance equal to the sum of all token balances.
    pub token_balances: TokenBalances,
}

impl LiquiditySwapContractState {
    /// Checks whether the state is valid.
    pub fn is_valid(&self) -> bool {
        self.is_valid_or_reason().is_ok()
    }

    /// Checks whether the state is valid, if not it will return an error reason.
    pub fn is_valid_or_reason(&self) -> Result<(), &'static str> {
        self.token_balances.is_valid_or_reason()?;
        if !ALLOWED_FEE_PER_MILLE.contains(&self.swap_fee_per_mille) {
            return Result::Err("Swap fee must be in range [0,1000]");
        }
        Result::Ok(())
    }

    /// Checks that the pools of the contracts have liquidity.
    ///
    /// ### Parameters:
    ///
    ///  * `state`: [`&LiquiditySwapContractState`] - A reference to the current state of the contract.
    ///
    /// ### Returns:
    /// True if the pools have liquidity, false otherwise [`bool`]
    fn contract_pools_have_liquidity(&self) -> bool {
        let contract_token_balance = self
            .token_balances
            .get_balance_for(&self.liquidity_pool_address);
        contract_token_balance.a_tokens != 0 && contract_token_balance.b_tokens != 0
    }
}

/// Initialize the contract.
///
/// # Parameters
///
///   * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///   * `token_a_address`: [`Address`] - The address of token A.
///
///   * `token_b_address`: [`Address`] - The address of token B.
///
///   * `swap_fee_per_mille`: [`TokenAmount`] - The fee for swapping, in per mille, i.e. a fee set to 3 corresponds to a fee of 0.3%.
///
///
/// The new state object of type [`LiquiditySwapContractState`] with all address fields initialized to their final state and remaining fields initialized to a default value.
///
#[init]
pub fn initialize(
    context: ContractContext,
    token_a_address: Address,
    token_b_address: Address,
    swap_fee_per_mille: u16,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let new_state = LiquiditySwapContractState {
        liquidity_pool_address: context.contract_address,
        swap_fee_per_mille,
        token_balances: TokenBalances::new(
            context.contract_address,
            token_a_address,
            token_b_address,
        ),
    };

    if let Err(msg) = new_state.is_valid_or_reason() {
        panic!("Cannot initialize contract: {}", msg);
    }

    (new_state, vec![])
}

/// Deposit token {A, B} into the calling user's balance on the contract.
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_address`: [`Address`] - The address of the deposited token contract.
///
///  * `amount`: [`TokenAmount`] - The amount to deposit.
///
/// # Returns
/// The unchanged state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x01)]
pub fn deposit(
    context: ContractContext,
    state: LiquiditySwapContractState,
    token_address: Address,
    amount: TokenAmount,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let tokens = state.token_balances.deduce_tokens_in_out(token_address);

    let mut event_group_builder = EventGroup::builder();
    interact_mpc20::MPC20Contract::at_address(token_address).transfer_from(
        &mut event_group_builder,
        &context.sender,
        &state.liquidity_pool_address,
        amount,
    );

    event_group_builder
        .with_callback(SHORTNAME_DEPOSIT_CALLBACK)
        .argument(tokens.token_in)
        .argument(amount)
        .done();

    (state, vec![event_group_builder.build()])
}

/// Handles callback from [`deposit`]. <br>
/// If the transfer event is successful,
/// the caller of [`deposit`] is registered as a user of the contract with (additional) `amount` added to their balance.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`] - The contractContext for the callback.
///
/// * `callback_context`: [`CallbackContext`] - The callbackContext.
///
/// * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
/// * `token`: [`Token`] - Indicating the token of which to add `amount` to.
///
/// * `amount`: [`TokenAmount`] - The desired amount to add to the user's total amount of `token`.
/// ### Returns
///
/// The updated state object of type [`LiquiditySwapContractState`] with an updated entry for the caller of `deposit`.
#[callback(shortname = 0x10)]
pub fn deposit_callback(
    context: ContractContext,
    callback_context: CallbackContext,
    mut state: LiquiditySwapContractState,
    token: Token,
    amount: TokenAmount,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    assert!(callback_context.success, "Transfer did not succeed");

    state
        .token_balances
        .add_to_token_balance(context.sender, token, amount);

    (state, vec![])
}

/// Swap <em>amount</em> of token A or B to the output token at the exchange rate dictated by <em>the constant product formula</em>.
/// The swap is executed on the token balances for the calling user.
///
/// The action will fail when:
///
/// - The contract does not have any liquidity.
/// - The caller does not have sufficient input token balance.
/// - The amount of output tokens is less than minimum specified (`amount_out_minimum`).
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_address`: [`Address`] - The address of the token contract being swapped from.
///
///  * `amount_in`: [`TokenAmount`] - The amount to swap of the token matching `input_token`.
///
///  * `amount_out_minimum`: [`TokenAmount`] - The minimum allowed amount of output tokens from the
///    swap. Should basically never be `0`, and should preferably be computed client-side with
///    a set amount of allowed slippage.
///
/// # Returns
/// The updated state object of type [`LiquiditySwapContractState`] yielding the result of the swap.
#[action(shortname = 0x02)]
pub fn swap(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_in: Address,
    amount_in: TokenAmount,
    amount_out_minimum: TokenAmount,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    assert!(
        state.contract_pools_have_liquidity(),
        "Pools must have existing liquidity to perform a swap"
    );

    let tokens = state.token_balances.deduce_tokens_in_out(token_in);
    let contract_token_balance = state
        .token_balances
        .get_balance_for(&state.liquidity_pool_address);

    let amount_out = calculate_swap_to_amount(
        contract_token_balance.get_amount_of(tokens.token_in),
        contract_token_balance.get_amount_of(tokens.token_out),
        amount_in,
        state.swap_fee_per_mille,
    );

    if amount_out < amount_out_minimum {
        panic!(
            "Swap produced {} output tokens, but minimum was set to {}.",
            amount_out, amount_out_minimum
        );
    }

    state.token_balances.move_tokens(
        context.sender,
        state.liquidity_pool_address,
        tokens.token_in,
        amount_in,
    );
    state.token_balances.move_tokens(
        state.liquidity_pool_address,
        context.sender,
        tokens.token_out,
        amount_out,
    );
    (state, vec![])
}

/// Withdraw <em>amount</em> of token {A, B} from the contract for the calling user.
/// This fails if `amount` is larger than the token balance of the corresponding token.
///
/// It preemptively updates the state of the user's balance before making the transfer.
/// This means that if the transfer fails, the contract could end up with more money than it has registered, which is acceptable.
/// This is to incentivize the user to spend enough gas to complete the transfer.
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_address`: [`Address`] - The address of the token contract to withdraw to.
///
///  * `amount`: [`TokenAmount`] - The amount to withdraw.
///
/// # Returns
/// The unchanged state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x03)]
pub fn withdraw(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_address: Address,
    amount: TokenAmount,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let tokens = state.token_balances.deduce_tokens_in_out(token_address);

    state
        .token_balances
        .deduct_from_token_balance(context.sender, tokens.token_in, amount);

    let mut event_group_builder = EventGroup::builder();
    interact_mpc20::MPC20Contract::at_address(token_address).transfer(
        &mut event_group_builder,
        &context.sender,
        amount,
    );

    (state, vec![event_group_builder.build()])
}

/// Become a liquidity provider to the contract by providing `amount` of tokens from the caller's balance. <br>
/// An equivalent amount of the output token is required to succeed and will be token_in implicitly. <br>
/// This is the inverse of [`reclaim_liquidity`].
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_address`: [`Address`] - The address of the input token.
///
///  * `token_amount`: [`TokenAmount`] - The amount to provide.
///
/// # Returns
/// The unchanged state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x04)]
pub fn provide_liquidity(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_address: Address,
    amount: TokenAmount,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let user = &context.sender;
    let tokens = state.token_balances.deduce_tokens_in_out(token_address);
    let contract_token_balance = state
        .token_balances
        .get_balance_for(&state.liquidity_pool_address);

    let (token_out_equivalent, minted_liquidity_tokens) = calculate_equivalent_and_minted_tokens(
        amount,
        contract_token_balance.get_amount_of(tokens.token_in),
        contract_token_balance.get_amount_of(tokens.token_out),
        contract_token_balance.liquidity_tokens,
    );
    assert!(
        minted_liquidity_tokens > 0,
        "The given input amount yielded 0 minted liquidity"
    );

    provide_liquidity_internal(
        &mut state,
        user,
        tokens,
        amount,
        token_out_equivalent,
        minted_liquidity_tokens,
    );
    (state, vec![])
}

/// Reclaim a calling user's share of the contract's total liquidity based on `liquidity_token_amount`. <br>
/// This is the inverse of [`provide_liquidity`].
///
/// Liquidity tokens are synonymous to weighted shares of the contract's total liquidity. <br>
/// As such, we calculate how much to output of token A and B,
/// based on the ratio between the input liquidity token amount and the total amount of liquidity minted by the contract.
///
/// ### Parameters:
///
/// * `context`: [`ContractContext`] - The context for the action call.
///
/// * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
/// * `liquidity_token_amount`: [`TokenAmount`] - The amount of liquidity tokens to burn.
///
/// ### Returns
///
/// The updated state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x05)]
pub fn reclaim_liquidity(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    liquidity_token_amount: TokenAmount,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let user = &context.sender;

    state
        .token_balances
        .deduct_from_token_balance(*user, Token::LIQUIDITY, liquidity_token_amount);

    let contract_token_balance = state
        .token_balances
        .get_balance_for(&state.liquidity_pool_address);

    let (a_output, b_output) = calculate_reclaim_output(
        liquidity_token_amount,
        contract_token_balance.a_tokens,
        contract_token_balance.b_tokens,
        contract_token_balance.liquidity_tokens,
    );

    state
        .token_balances
        .move_tokens(state.liquidity_pool_address, *user, Token::A, a_output);
    state
        .token_balances
        .move_tokens(state.liquidity_pool_address, *user, Token::B, b_output);
    state.token_balances.deduct_from_token_balance(
        state.liquidity_pool_address,
        Token::LIQUIDITY,
        liquidity_token_amount,
    );

    (state, vec![])
}

/// Initialize token liquidity pools, and mint initial liquidity tokens.
///
/// Calling this action makes the calling user the first liquidity provider, receiving liquidity
/// tokens amounting to 100% of the contract's total liquidity, until another user becomes an
/// liquidity provider.
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_a_amount`: [`TokenAmount`] - The amount to initialize pool A with.
///
///  * `token_b_amount`: [`TokenAmount`] - The amount to initialize pool B with.
///
/// # Returns
///
/// The updated state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x06)]
pub fn provide_initial_liquidity(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_a_amount: TokenAmount,
    token_b_amount: TokenAmount,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    assert!(
        !state.contract_pools_have_liquidity(),
        "Can only initialize when both pools are empty"
    );

    let minted_liquidity_tokens = initial_liquidity_tokens(token_a_amount, token_b_amount);
    assert!(
        minted_liquidity_tokens > 0,
        "The given input amount yielded 0 minted liquidity"
    );

    provide_liquidity_internal(
        &mut state,
        &context.sender,
        TokensInOut::A_IN_B_OUT,
        token_a_amount,
        token_b_amount,
        minted_liquidity_tokens,
    );
    (state, vec![])
}

/// Determines the initial amount of liquidity tokens, or shares, representing some sensible '100%' of the contract's liquidity. <br>
/// This implementation is derived from section 3.4 of: [Uniswap v2 whitepaper](https://uniswap.org/whitepaper.pdf). <br>
/// It guarantees that the value of a liquidity token becomes independent of the ratio at which liquidity was initially token_in.
fn initial_liquidity_tokens(
    token_a_amount: TokenAmount,
    token_b_amount: TokenAmount,
) -> TokenAmount {
    math::u128_sqrt(token_a_amount * token_b_amount).into()
}

/// Calculates how many of the output token you can get for `swap_amount_in` given an exchange fee in per mille. <br>
/// In other words, calculates how much the input token amount, minus the fee, is worth in the output token currency. <br>
/// This calculation is derived from section 3.1.2 of [UniSwap v1 whitepaper](https://github.com/runtimeverification/verified-smart-contracts/blob/uniswap/uniswap/x-y-k.pdf)
///
/// ### Parameters:
///
/// * `pool_token_in`: [`TokenAmount`] - The token pool matching the token of `swap_amount_in`.
///
/// * `pool_token_out`: [`TokenAmount`] - The output token pool.
///
/// * `swap_amount_in`: [`TokenAmount`] - The amount being swapped.
///
/// * `swap_fee_per_mille`: [`u16`] - The fee to take out of swapped to amount. Must be in [`ALLOWED_FEE_PER_MILLE`].
///
/// # Returns
/// The amount received after swapping. [`TokenAmount`]
fn calculate_swap_to_amount(
    pool_token_in: TokenAmount,
    pool_token_out: TokenAmount,
    swap_amount_in: TokenAmount,
    swap_fee_per_mille: u16,
) -> TokenAmount {
    let remainder_ratio = (1000 - swap_fee_per_mille) as TokenAmount;
    (remainder_ratio * swap_amount_in * pool_token_out)
        / (1000 * pool_token_in + remainder_ratio * swap_amount_in)
}

/// Finds the equivalent value of the output token during [`provide_liquidity`] based on the input amount and the weighted shares that they correspond to. <br>
/// Due to integer rounding, a user may be depositing an additional token and mint one less than expected. <br>
/// Calculations are derived from section 2.1.2 of [UniSwap v1 whitepaper](https://github.com/runtimeverification/verified-smart-contracts/blob/uniswap/uniswap/x-y-k.pdf)
///
/// ### Parameters:
///
/// * `token_in_amount`: [`TokenAmount`] - The amount being token_in to the contract.
///
/// * `token_in_pool`: [`TokenAmount`] - The token pool matching the input token.
///
/// * `token_out_pool`: [`TokenAmount`] - The token_out pool.
///
/// * `total_minted_liquidity` [`TokenAmount`] - The total current minted liquidity.
/// # Returns
/// The new A pool, B pool and minted liquidity values ([`TokenAmount`], [`TokenAmount`], [`TokenAmount`])
fn calculate_equivalent_and_minted_tokens(
    token_in_amount: TokenAmount,
    token_in_pool: TokenAmount,
    token_out_pool: TokenAmount,
    total_minted_liquidity: TokenAmount,
) -> (TokenAmount, TokenAmount) {
    // Handle zero-case
    let token_out_equivalent = if token_in_amount > 0 {
        (token_in_amount * token_out_pool / token_in_pool) + 1
    } else {
        0
    };
    let minted_liquidity_tokens = token_in_amount * total_minted_liquidity / token_in_pool;
    (token_out_equivalent, minted_liquidity_tokens)
}

/// Calculates the amount of token {A, B} that the input amount of liquidity tokens correspond to during [`reclaim_liquidity`]. <br>
/// Due to integer rounding, a user may be withdrawing less of each pool token than expected. <br>
/// Calculations are derived from section 2.2.2 of [UniSwap v1 whitepaper](
/// https://github.com/runtimeverification/verified-smart-contracts/blob/uniswap/uniswap/x-y-k.pdf)
///
/// ### Parameters:
///
/// * `liquidity_token_amount`: [`TokenAmount`] - The amount of liquidity tokens being reclaimed.
///
/// * `pool_a`: [`TokenAmount`] - Pool a of this contract.
///
/// * `pool_b`: [`TokenAmount`] - Pool b of this contract.
///
/// * `minted_liquidity` [`TokenAmount`] - The total current minted liquidity.
/// # Returns
/// The new A pool, B pool and minted liquidity values ([`TokenAmount`], [`TokenAmount`], [`TokenAmount`])
fn calculate_reclaim_output(
    liquidity_token_amount: TokenAmount,
    pool_a: TokenAmount,
    pool_b: TokenAmount,
    minted_liquidity: TokenAmount,
) -> (TokenAmount, TokenAmount) {
    let a_output = pool_a * liquidity_token_amount / minted_liquidity;
    let b_output = pool_b * liquidity_token_amount / minted_liquidity;
    (a_output, b_output)
}

/// Moves tokens from the providing user's balance to the contract's and mints liquidity tokens.
///
/// ### Parameters:
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
/// * `user`: [`&Address`] - The address of the user providing liquidity.
///
/// * `token_in`: [`Address`] - The address of the token being token_in.
///
///  * `token_in_amount`: [`TokenAmount`] - The input token amount.
///
///  * `token_out_amount`: [`TokenAmount`] - The output token amount. Must be equal value to `token_in_amount` at the current exchange rate.
///
///  * `minted_liquidity_tokens`: [`TokenAmount`] - The amount of liquidity tokens that the input tokens yields.
fn provide_liquidity_internal(
    state: &mut LiquiditySwapContractState,
    user: &Address,
    tokens: TokensInOut,
    token_in_amount: TokenAmount,
    token_out_amount: TokenAmount,
    minted_liquidity_tokens: TokenAmount,
) {
    state.token_balances.move_tokens(
        *user,
        state.liquidity_pool_address,
        tokens.token_in,
        token_in_amount,
    );
    state.token_balances.move_tokens(
        *user,
        state.liquidity_pool_address,
        tokens.token_out,
        token_out_amount,
    );

    state
        .token_balances
        .add_to_token_balance(*user, Token::LIQUIDITY, minted_liquidity_tokens);
    state.token_balances.add_to_token_balance(
        state.liquidity_pool_address,
        Token::LIQUIDITY,
        minted_liquidity_tokens,
    );
}
