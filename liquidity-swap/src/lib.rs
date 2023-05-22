//! This is an example liquidity swap smart contract. <br>
//! It is based on [UniSwap v1](https://hackmd.io/@HaydenAdams/HJ9jLsfTz?type=view) <br>
//!
//! The contracts exchanges (or swaps) between two types of tokens, <br>
//! with an the exchange rate as given by the `constant product formula: x * y = k`. <br>
//! We consider `x` to be the balance of token pool A and `y` to be the balance of token pool B and `k` to be their product. <br>
//! When performing a swap, a fee of 0.3% is applied, based on the input amount, which is deducted from the output of the swap. <br>
//! This effectively increases `k` after each swap.<br><br>
//!
//! In order to perform a swap, it is a prerequisite that the swapping user has already transferred
//! at least one of the tokens to the contract via a call to [`deposit`]. <br>
//! Additionally, some user (typically the creator of the contract) must have already deposited an amount of both token types and initialized both pools by a call to [`provide_initial_liquidity`]. <br><br>
//!
//! A user may [`withdraw`] the resulting tokens of a swap (or simply his own deposited tokens)
//! to have the tokens transferred to his account, at any point.<br><br>
//!
//! Finally, a user may choose to become a liquidity provider (LP) of the contract
//! by providing an amount of pre-deposited tokens taken from the user's internal token balance.
//! This yields the LP a share of the contract's total liquidity, based on the ratio between the amount of provided liquidity and the contract's total liquidity at the time of providing. <br>
//! These shares are referred to as `liquidity tokens` which are minted upon becoming an LP and may later be burned to receive a proportionate share of the contract's liquidity. <br>
//! Since `k` increases between swaps, an LP stands to profit from burning their liquidity token after x amount of swaps has occurred.<br>
//! The larger the shares an LP has, the larger the profit. <br>
//! However, as with all investing, an LP also risks losing profit if the market-clearing price of at least one of the tokens decreases to a point that exceeds the rewards gained from swap-fees.<br><br>
//! Since liquidity tokens represent an equal share of both tokens, when providing liquidity it is enforced that the user provides an equivalent value of the opposite token to the tokens provided. <br><br>
//!
//! Because the relative price of the two tokens can only be changed through swapping,
//! divergences between the prices of the contract and the prices of similar external contracts create arbitrage opportunities.
//! This mechanism ensures that the contract's prices always trend toward the market-clearing price.
//!
#![allow(unused_variables)]

mod tests;

#[macro_use]
extern crate pbc_contract_codegen;
extern crate core;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::{Address, AddressType, Shortname};
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Enum for token types
#[derive(PartialEq, Eq, ReadWriteRPC, CreateTypeSpec)]
#[cfg_attr(test, derive(Debug))]
pub enum Token {
    /// The value representing token A.
    #[discriminant(0)]
    TokenA {},
    /// The value representing token B.
    #[discriminant(1)]
    TokenB {},
    /// The value representing a liquidity token.
    #[discriminant(2)]
    LiquidityToken {},
}

/// Make reference to tokens more readable
impl Token {
    const A: Token = Token::TokenA {};
    const B: Token = Token::TokenB {};
    const LIQUIDITY: Token = Token::LiquidityToken {};
}

/// Keeps track of how much of a given token a user owns within the scope of the contract.
#[derive(ReadWriteState, CreateTypeSpec)]
#[cfg_attr(test, derive())]
pub struct TokenBalance {
    /// The amount of token A that a user can withdraw from the contract.
    pub a_tokens: u128,
    /// The amount of token B that a user can withdraw from the contract.
    pub b_tokens: u128,
    /// The amount of liquidity tokens that a user may burn.
    pub liquidity_tokens: u128,
}

impl TokenBalance {
    /// Retrieves a copy of the amount that matches `token`.
    ///
    /// ### Parameters:
    ///
    /// * `token`: [`Token`] - The token matching the desired amount.
    ///
    /// # Returns
    /// A value of type [`u128`]
    fn get_amount_of(&self, token: &Token) -> u128 {
        if token == &Token::LIQUIDITY {
            self.liquidity_tokens
        } else if token == &Token::A {
            self.a_tokens
        } else {
            self.b_tokens
        }
    }

    /// Retrieves a mutable reference to the amount that matches `token`.
    ///
    /// ### Parameters:
    ///
    /// * `token`: [`Token`] - The token matching the desired amount.
    ///
    /// # Returns
    /// A mutable value of type [`&mut u128`]
    fn get_mut_amount_of(&mut self, token: &Token) -> &mut u128 {
        if token == &Token::LIQUIDITY {
            &mut self.liquidity_tokens
        } else if token == &Token::A {
            &mut self.a_tokens
        } else {
            &mut self.b_tokens
        }
    }

    /// Checks that the user has no tokens.
    ///
    /// ### Returns:
    /// True if the user has no tokens, false otherwise [`bool`]
    fn user_has_no_tokens(&self) -> bool {
        self.a_tokens == 0 && self.b_tokens == 0 && self.liquidity_tokens == 0
    }
}

/// Empty token balance.
const EMPTY_BALANCE: TokenBalance = TokenBalance {
    a_tokens: 0,
    b_tokens: 0,
    liquidity_tokens: 0,
};

/// This is the state of the contract which is persisted on the chain.
///
/// The #\[state\] macro generates serialization logic for the struct.
#[state]
pub struct LiquiditySwapContractState {
    /// The address of this contract
    pub contract: Address,
    /// The address of the first token.
    pub token_a_address: Address,
    /// The address of the second token.
    pub token_b_address: Address,
    /// The fee for making swaps per mille.
    pub swap_fee_per_mille: u128,
    /// The map containing all token balances of all users and the contract itself. <br>
    /// The contract should always have a balance equal to the sum of all token balances.
    pub token_balances: SortedVecMap<Address, TokenBalance>,
}

impl LiquiditySwapContractState {
    /// Adds tokens to the `token_balances` map of the contract. <br>
    /// If the user isn't already present, creates an entry with an empty TokenBalance.
    ///
    /// ### Parameters:
    ///
    /// * `user`: [`&Address`] - A reference to the user to add `amount` to.
    ///
    /// * `token`: [`Token`] - The token to add to.
    ///
    /// * `amount`: [`u128`] - The amount to add.
    ///
    fn add_to_token_balance(&mut self, user: Address, token: Token, amount: u128) {
        let token_balance = self.get_mut_balance_for(&user);
        *token_balance.get_mut_amount_of(&token) += amount;
    }

    /// Deducts tokens from the `token_balances` map of the contract. <br>
    /// Requires that the user has at least as many tokens as is being deducted.
    ///
    /// ### Parameters:
    ///
    /// * `user`: [`&Address`] - A reference to the user to deduct `amount` from.
    ///
    /// * `token`: [`Token`] - The token to subtract from.
    ///
    /// * `amount`: [`u128`] - The amount to subtract.
    ///
    fn deduct_from_token_balance(&mut self, user: Address, token: &Token, amount: u128) {
        let token_balance = self.get_mut_balance_for(&user);
        *token_balance.get_mut_amount_of(token) = token_balance
            .get_amount_of(token)
            .checked_sub(amount)
            .expect("Insufficient funds");

        if token_balance.user_has_no_tokens() {
            self.token_balances.remove(&user);
        }
    }

    /// Moves internal tokens from the `from`-address to the `to`-address.
    ///
    /// ### Parameters:
    ///
    /// * `from`: [`Address`] - The address of the transferring party.
    ///
    /// * `to`: [`Address`] - The address of the receiving party.
    ///
    /// * `moved_token`: [`Token`] - The token being transferred.
    ///
    /// * `amount`: [`u128`] - The amount being transferred.
    ///
    fn move_tokens(&mut self, from: Address, to: Address, moved_token: Token, amount: u128) {
        self.deduct_from_token_balance(from, &moved_token, amount);
        self.add_to_token_balance(to, moved_token, amount);
    }

    /// Retrieves a copy of the token balance that matches `user`.
    ///
    /// ### Parameters:
    ///
    /// * `user`: [`&Address`] - A reference to the desired user address.
    ///
    /// # Returns
    /// A copy of the token balance that matches `user`.
    fn get_balance_for(&self, user: &Address) -> &TokenBalance {
        let token_balance = self.token_balances.get(user).unwrap_or(&EMPTY_BALANCE);
        token_balance
    }

    /// Retrieves a mutable reference to the token balance that matches `user`.
    ///
    /// ### Parameters:
    ///
    /// * `user`: [`&Address`] - A reference to the desired user address.
    ///
    /// # Returns
    /// The mutable reference to the token balance that matches `user`.
    fn get_mut_balance_for(&mut self, user: &Address) -> &mut TokenBalance {
        if !self.token_balances.contains_key(user) {
            self.token_balances.insert(*user, EMPTY_BALANCE);
        }
        self.token_balances.get_mut(user).unwrap()
    }

    /// Retrieves a pair of tokens with the `provided_token_address` being the "provided"-token
    /// and the remaining token being "opposite". <br>
    /// Requires that `provided_token_address` matches the contract's pools.
    ///
    /// ### Parameters:
    ///
    /// * `provided_token_address`: [`Token`] - The desired token to work with.
    ///
    /// # Returns
    /// The provided/opposite-pair of tokens of type [`(Token, Token)`]
    fn deduce_provided_opposite_tokens(&self, provided_token_address: Address) -> (Token, Token) {
        let provided_a = self.token_a_address == provided_token_address;
        let provided_b = self.token_b_address == provided_token_address;
        if !provided_a && !provided_b {
            panic!("Provided invalid token address")
        }

        if provided_a {
            (Token::A, Token::B)
        } else {
            (Token::B, Token::A)
        }
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
        let contract_token_balance = self.get_balance_for(&self.contract);
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
///   * `swap_fee_per_mille`: [`u128`] - The fee for swapping, in per mille, i.e. a fee set to 3 corresponds to a fee of 0.3%.
///
///
/// The new state object of type [`LiquiditySwapContractState`] with all address fields initialized to their final state and remaining fields initialized to a default value.
///
#[init]
pub fn initialize(
    context: ContractContext,
    token_a_address: Address,
    token_b_address: Address,
    swap_fee_per_mille: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    assert_ne!(
        token_a_address.address_type,
        AddressType::Account,
        "Tried to provide an account as token for token A"
    );
    assert_ne!(
        token_b_address.address_type,
        AddressType::Account,
        "Tried to provide an account as token for token B"
    );
    assert_ne!(
        token_a_address, token_b_address,
        "Cannot initialize swap with duplicate tokens"
    );
    assert!(
        swap_fee_per_mille <= 1000,
        "Swap fee should not exceed 1000"
    );

    let new_state = LiquiditySwapContractState {
        contract: context.contract_address,
        token_a_address,
        token_b_address,
        swap_fee_per_mille,
        token_balances: SortedVecMap::new(),
    };

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
///  * `amount`: [`u128`] - The amount to deposit.
///
/// # Returns
/// The unchanged state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x01)]
pub fn deposit(
    context: ContractContext,
    state: LiquiditySwapContractState,
    token_address: Address,
    amount: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let (from_token, _) = state.deduce_provided_opposite_tokens(token_address);
    let mut event_group_builder = EventGroup::builder();
    event_group_builder
        .call(token_address, token_contract_transfer_from())
        .argument(context.sender)
        .argument(context.contract_address)
        .argument(amount)
        .done();

    event_group_builder
        .with_callback(SHORTNAME_DEPOSIT_CALLBACK)
        .argument(from_token)
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
/// * `amount`: [`u128`] - The desired amount to add to the user's total amount of `token`.
/// ### Returns
///
/// The updated state object of type [`LiquiditySwapContractState`] with an updated entry for the caller of `deposit`.
#[callback(shortname = 0x10)]
pub fn deposit_callback(
    context: ContractContext,
    callback_context: CallbackContext,
    mut state: LiquiditySwapContractState,
    token: Token,
    amount: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    assert!(callback_context.success, "Transfer did not succeed");

    state.add_to_token_balance(context.sender, token, amount);

    (state, vec![])
}

/// <pre>
/// Swap <em>amount</em> of token A or B to the opposite token at the exchange rate dictated by <em>the constant product formula</em>.
/// The swap is executed on the token balances for the calling user.
/// If the contract has empty pools or if the caller does not have a sufficient balance of the token, the action fails.
/// </pre>
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_address`: [`Address`] - The address of the token contract being swapped from.
///
///  * `amount`: [`u128`] - The amount to swap of the token matching `input_token`.
///
/// # Returns
/// The updated state object of type [`LiquiditySwapContractState`] yielding the result of the swap.
#[action(shortname = 0x02)]
pub fn swap(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_address: Address,
    amount: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    assert!(
        state.contract_pools_have_liquidity(),
        "Pools must have existing liquidity to perform a swap"
    );

    let (provided_token, opposite_token) = state.deduce_provided_opposite_tokens(token_address);
    let contract_token_balance = state.get_balance_for(&state.contract);

    let opposite_token_amount = calculate_swap_to_amount(
        contract_token_balance.get_amount_of(&provided_token),
        contract_token_balance.get_amount_of(&opposite_token),
        amount,
        state.swap_fee_per_mille,
    );

    state.move_tokens(context.sender, state.contract, provided_token, amount);
    state.move_tokens(
        state.contract,
        context.sender,
        opposite_token,
        opposite_token_amount,
    );
    (state, vec![])
}

/// <pre>
/// Withdraw <em>amount</em> of token {A, B} from the contract for the calling user.
/// This fails if `amount` is larger than the token balance of the corresponding token.
///
/// It preemptively updates the state of the user's balance before making the transfer.
/// This means that if the transfer fails, the contract could end up with more money than it has registered, which is acceptable.
/// This is to incentivize the user to spend enough gas to complete the transfer.
/// </pre>
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_address`: [`Address`] - The address of the token contract to withdraw to.
///
///  * `amount`: [`u128`] - The amount to withdraw.
///
/// # Returns
/// The unchanged state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x03)]
pub fn withdraw(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_address: Address,
    amount: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let (provided_token, _) = state.deduce_provided_opposite_tokens(token_address);

    state.deduct_from_token_balance(context.sender, &provided_token, amount);

    let mut event_group_builder = EventGroup::builder();
    event_group_builder
        .call(token_address, token_contract_transfer())
        .argument(context.sender)
        .argument(amount)
        .done();

    (state, vec![event_group_builder.build()])
}

/// Become a liquidity provider to the contract by providing `amount` of tokens from the caller's balance. <br>
/// An equivalent amount of the opposite token is required to succeed and will be provided implicitly. <br>
/// This is the inverse of [`reclaim_liquidity`].
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_address`: [`Address`] - The address of the provided token.
///
///  * `token_amount`: [`u128`] - The amount to provide.
///
/// # Returns
/// The unchanged state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x04)]
pub fn provide_liquidity(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_address: Address,
    amount: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let user = &context.sender;
    let (provided_token, opposite_token) = state.deduce_provided_opposite_tokens(token_address);
    let contract_token_balance = state.get_balance_for(&state.contract);

    let (opposite_equivalent, minted_liquidity_tokens) = calculate_equivalent_and_minted_tokens(
        amount,
        contract_token_balance.get_amount_of(&provided_token),
        contract_token_balance.get_amount_of(&opposite_token),
        contract_token_balance.liquidity_tokens,
    );
    assert!(
        minted_liquidity_tokens > 0,
        "Provided amount yielded 0 minted liquidity"
    );

    provide_liquidity_internal(
        &mut state,
        user,
        token_address,
        amount,
        opposite_equivalent,
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
/// * `liquidity_token_amount`: [`u128`] - The amount of liquidity tokens to burn.
///
/// ### Returns
///
/// The updated state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x05)]
pub fn reclaim_liquidity(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    liquidity_token_amount: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    let user = &context.sender;

    state.deduct_from_token_balance(*user, &Token::LIQUIDITY, liquidity_token_amount);

    let contract_token_balance = state.get_balance_for(&state.contract);

    let (a_output, b_output) = calculate_reclaim_output(
        liquidity_token_amount,
        contract_token_balance.a_tokens,
        contract_token_balance.b_tokens,
        contract_token_balance.liquidity_tokens,
    );

    state.move_tokens(state.contract, *user, Token::A, a_output);
    state.move_tokens(state.contract, *user, Token::B, b_output);
    state.deduct_from_token_balance(state.contract, &Token::LIQUIDITY, liquidity_token_amount);

    (state, vec![])
}

/// <pre>
/// Initialize pool {A, B} of the contract and mint initial liquidity tokens.
/// This effectively makes the calling user the first LP,
/// receiving liquidity tokens amounting to 100% of the contract's total liquidity,
/// until another user becomes an LP.</pre>
///
/// ### Parameters:
///
///  * `context`: [`ContractContext`] - The contract context containing sender and chain information.
///
///  * `state`: [`LiquiditySwapContractState`] - The current state of the contract.
///
///  * `token_a_amount`: [`u128`] - The amount to initialize pool A with.
///
///  * `token_b_amount`: [`u128`] - The amount to initialize pool B with.
///
/// # Returns
/// The updated state object of type [`LiquiditySwapContractState`].
#[action(shortname = 0x06)]
pub fn provide_initial_liquidity(
    context: ContractContext,
    mut state: LiquiditySwapContractState,
    token_a_amount: u128,
    token_b_amount: u128,
) -> (LiquiditySwapContractState, Vec<EventGroup>) {
    assert!(
        !state.contract_pools_have_liquidity(),
        "Can only initialize when both pools are empty"
    );

    let minted_liquidity_tokens = initial_liquidity_tokens(token_a_amount, token_b_amount);
    assert!(
        minted_liquidity_tokens > 0,
        "Provided amount yielded 0 minted liquidity"
    );

    let provided_address = state.token_a_address;
    provide_liquidity_internal(
        &mut state,
        &context.sender,
        provided_address,
        token_a_amount,
        token_b_amount,
        minted_liquidity_tokens,
    );
    (state, vec![])
}

/// Determines the initial amount of liquidity tokens, or shares, representing some sensible '100%' of the contract's liquidity. <br>
/// This implementation is derived from section 3.4 of: [Uniswap v2 whitepaper](https://uniswap.org/whitepaper.pdf). <br>
/// It guarantees that the value of a liquidity token becomes independent of the ratio at which liquidity was initially provided.
fn initial_liquidity_tokens(token_a_amount: u128, token_b_amount: u128) -> u128 {
    u128_sqrt(token_a_amount * token_b_amount)
}

/// Creates the `Shortname` corresponding to the `transfer` action of a token contract. <br>
/// This is utilized in combination with an `EventGroupBuilder`'s `call` function.
///
/// ### Returns:
///
/// The `Shortname` corresponding to the `transfer` action of a token contract.
#[inline]
fn token_contract_transfer() -> Shortname {
    Shortname::from_u32(0x01)
}

/// Creates the `Shortname` corresponding to the `transfer_from` action of a token contract. <br>
/// This is utilized in combination with an `EventGroupBuilder`'s `call` function.
///
/// ### Returns:
///
/// The `Shortname` corresponding to the `transfer_from` action of a token contract.
#[inline]
fn token_contract_transfer_from() -> Shortname {
    Shortname::from_u32(0x03)
}

/// Find the u128 square root of `y` (using binary search) rounding down.
///
/// ### Parameters:
///
/// * `y`: [`u128`] - The number to find the square root of.
///
/// ### Returns:
/// The largest x, such that x*x is <= y of type [`u128`]
fn u128_sqrt(y: u128) -> u128 {
    let mut l: u128 = 0;
    let mut m: u128;
    let mut r: u128 = y + 1;

    while l != r - 1 {
        m = (l + r) / 2; // binary search (round down)

        if m * m <= y {
            l = m; // Keep searching in right side
        } else {
            r = m; // Keep searching in left side
        }
    }
    l
}

/// Calculates how many of the opposite token you can get for `swap_from_amount` given an exchange fee in per mille. <br>
/// In other words, calculates how much the input token amount, minus the fee, is worth in the opposite token currency. <br>
/// This calculation is derived from section 3.1.2 of [UniSwap v1 whitepaper](https://github.com/runtimeverification/verified-smart-contracts/blob/uniswap/uniswap/x-y-k.pdf)
///
/// ### Parameters:
///
/// * `from_pool`: [`u128`] - The token pool matching the token of `swap_from_amount`.
///
/// * `to_pool`: [`u128`] - The opposite token pool.
///
/// * `swap_from_amount`: [`u128`] - The amount being swapped.
/// # Returns
/// The amount received after swapping. [`u128`]
fn calculate_swap_to_amount(
    from_pool: u128,
    to_pool: u128,
    swap_from_amount: u128,
    swap_fee_per_mille: u128,
) -> u128 {
    let remainder_ratio = 1000 - swap_fee_per_mille;
    (remainder_ratio * swap_from_amount * to_pool)
        / (1000 * from_pool + remainder_ratio * swap_from_amount)
}

/// Finds the equivalent value of the opposite token during [`provide_liquidity`] based on the input amount and the weighted shares that they correspond to. <br>
/// Due to integer rounding, a user may be depositing an additional token and mint one less than expected. <br>
/// Calculations are derived from section 2.1.2 of [UniSwap v1 whitepaper](https://github.com/runtimeverification/verified-smart-contracts/blob/uniswap/uniswap/x-y-k.pdf)
///
/// ### Parameters:
///
/// * `provided_amount`: [`u128`] - The amount being provided to the contract.
///
/// * `provided_pool`: [`u128`] - The token pool matching the provided amount.
///
/// * `opposite_pool`: [`u128`] - The opposite pool.
///
/// * `total_minted_liquidity` [`u128`] - The total current minted liquidity.
/// # Returns
/// The new A pool, B pool and minted liquidity values ([`u128`], [`u128`], [`u128`])
fn calculate_equivalent_and_minted_tokens(
    provided_amount: u128,
    provided_pool: u128,
    opposite_pool: u128,
    total_minted_liquidity: u128,
) -> (u128, u128) {
    // Handle zero-case
    let opposite_equivalent = if provided_amount > 0 {
        (provided_amount * opposite_pool / provided_pool) + 1
    } else {
        0
    };
    let minted_liquidity_tokens = provided_amount * total_minted_liquidity / provided_pool;
    (opposite_equivalent, minted_liquidity_tokens)
}

/// Calculates the amount of token {A, B} that the input amount of liquidity tokens correspond to during [`reclaim_liquidity`]. <br>
/// Due to integer rounding, a user may be withdrawing less of each pool token than expected. <br>
/// Calculations are derived from section 2.2.2 of [UniSwap v1 whitepaper](
/// https://github.com/runtimeverification/verified-smart-contracts/blob/uniswap/uniswap/x-y-k.pdf)
///
/// ### Parameters:
///
/// * `liquidity_token_amount`: [`u128`] - The amount of liquidity tokens being reclaimed.
///
/// * `pool_a`: [`u128`] - Pool a of this contract.
///
/// * `pool_b`: [`u128`] - Pool b of this contract.
///
/// * `minted_liquidity` [`u128`] - The total current minted liquidity.
/// # Returns
/// The new A pool, B pool and minted liquidity values ([`u128`], [`u128`], [`u128`])
fn calculate_reclaim_output(
    liquidity_token_amount: u128,
    pool_a: u128,
    pool_b: u128,
    minted_liquidity: u128,
) -> (u128, u128) {
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
/// * `provided_token_address`: [`Address`] - The address of the token being provided.
///
///  * `provided_amount`: [`u128`] - The amount provided.
///
///  * `opposite_amount`: [`u128`] - The amount equivalent to the provided amount of the opposite token.
///
///  * `minted_liquidity_tokens`: [`u128`] - The amount of liquidity tokens that the provided tokens yields.
fn provide_liquidity_internal(
    state: &mut LiquiditySwapContractState,
    user: &Address,
    provided_token_address: Address,
    provided_amount: u128,
    opposite_amount: u128,
    minted_liquidity_tokens: u128,
) {
    let (provided_token, opposite_token) =
        state.deduce_provided_opposite_tokens(provided_token_address);

    state.move_tokens(*user, state.contract, provided_token, provided_amount);
    state.move_tokens(*user, state.contract, opposite_token, opposite_amount);

    state.add_to_token_balance(*user, Token::LIQUIDITY, minted_liquidity_tokens);
    state.add_to_token_balance(state.contract, Token::LIQUIDITY, minted_liquidity_tokens);
}
