//! Mini-library for generalized token balance data structure.

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::{Address, AddressType};
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use pbc_traits::ReadWriteState;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Type used to represent token amounts.
pub type TokenAmount = u128;

/// Enum for token types
#[derive(PartialEq, Eq, ReadWriteRPC, CreateTypeSpec, Clone, Copy, Debug)]
#[repr(u8)]
pub enum Token {
    /// The value representing token A.
    #[discriminant(0)]
    TokenA {},
    /// The value representing token B.
    #[discriminant(1)]
    TokenB {},
    /// The value representing liquidity token.
    #[discriminant(2)]
    LiquidityToken {},
}

/// Make reference to tokens more readable
impl Token {
    /// The value representing token A.
    pub const A: Token = Token::TokenA {};

    /// The value representing token B.
    pub const B: Token = Token::TokenB {};

    /// The value representing liquidity token.
    pub const LIQUIDITY: Token = Token::LiquidityToken {};
}

/// Keeps track of how much of a given token a user owns within the scope of the contract.
#[repr(align(1))]
#[derive(ReadWriteState, CreateTypeSpec, Debug, PartialEq, Clone)]
pub struct TokenBalance {
    /// The amount of token A that a user can withdraw from the contract.
    pub a_tokens: TokenAmount,
    /// The amount of token B that a user can withdraw from the contract.
    pub b_tokens: TokenAmount,
    /// The amount of liquidity tokens that a user may burn.
    pub liquidity_tokens: TokenAmount,
}

/// Checks that [`TokenBalance`] can be efficiently serialized using [`ReadWriteState`].
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(TokenBalance::SERIALIZABLE_BY_COPY);

impl TokenBalance {
    /// Retrieves a copy of the amount that matches `token`.
    ///
    /// ### Parameters:
    ///
    /// * `token`: [`Token`] - The token matching the desired amount.
    ///
    /// # Returns
    /// A value of type [`TokenAmount`]
    pub fn get_amount_of(&self, token: Token) -> TokenAmount {
        if token == Token::LIQUIDITY {
            self.liquidity_tokens
        } else if token == Token::A {
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
    /// A mutable value of type [`&mut TokenAmount`]
    fn get_mut_amount_of(&mut self, token: Token) -> &mut TokenAmount {
        if token == Token::LIQUIDITY {
            &mut self.liquidity_tokens
        } else if token == Token::A {
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

/// Generalized token balance structure.
#[derive(ReadWriteState, CreateTypeSpec, Debug, PartialEq, Clone)]
pub struct TokenBalances {
    /// The address of the liquidity tokens (likely this contract itself.)
    pub token_lp_address: Address,
    /// The address of the first token.
    pub token_a_address: Address,
    /// The address of the second token.
    pub token_b_address: Address,
    /// The map containing all token balances of all users and the contract itself. <br>
    /// The contract should always have a balance equal to the sum of all token balances.
    balances: SortedVecMap<Address, TokenBalance>,
}

impl TokenBalances {
    /// Creates new token balances structure from the given token addresses.
    pub fn new(
        token_lp_address: Address,
        token_a_address: Address,
        token_b_address: Address,
    ) -> Self {
        Self {
            token_lp_address,
            token_a_address,
            token_b_address,
            balances: SortedVecMap::new(),
        }
    }

    /// Checks whether the state is valid.
    pub fn is_valid(&self) -> bool {
        self.is_valid_or_reason().is_ok()
    }

    /// Checks whether the state is valid, if not it will return an error reason.
    pub fn is_valid_or_reason(&self) -> Result<(), &'static str> {
        if self.token_a_address.address_type == AddressType::Account {
            return Result::Err("Token address A must be a contract address");
        }
        if self.token_b_address.address_type == AddressType::Account {
            return Result::Err("Token address B must be a contract address");
        }
        if self.token_a_address == self.token_b_address {
            return Result::Err("Tokens A and B must not be the same contract");
        }
        Result::Ok(())
    }

    /// Adds tokens to the `balances` map of the contract. <br>
    /// If the user isn't already present, creates an entry with an empty TokenBalance.
    ///
    /// ### Parameters:
    ///
    /// * `user`: [`&Address`] - A reference to the user to add `amount` to.
    ///
    /// * `token`: [`Token`] - The token to add to.
    ///
    /// * `amount`: [`TokenAmount`] - The amount to add.
    ///
    pub fn add_to_token_balance(&mut self, user: Address, token: Token, amount: TokenAmount) {
        let token_balance = self.get_mut_balance_for(&user);
        *token_balance.get_mut_amount_of(token) += amount;
    }

    /// Deducts tokens from the `balances` map of the contract. <br>
    /// Requires that the user has at least as many tokens as is being deducted.
    ///
    /// ### Parameters:
    ///
    /// * `user`: [`&Address`] - A reference to the user to deduct `amount` from.
    ///
    /// * `token`: [`Token`] - The token to subtract from.
    ///
    /// * `amount`: [`TokenAmount`] - The amount to subtract.
    ///
    pub fn deduct_from_token_balance(&mut self, user: Address, token: Token, amount: TokenAmount) {
        let user_balances = self.get_mut_balance_for(&user);

        let token_balance = user_balances.get_amount_of(token);

        *user_balances.get_mut_amount_of(token) =
            token_balance.checked_sub(amount).unwrap_or_else(|| {
                panic!(
                    "Insufficient {:?} deposit: {}/{}",
                    token, token_balance, amount
                )
            });

        if user_balances.user_has_no_tokens() {
            self.balances.remove(&user);
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
    /// * `amount`: [`TokenAmount`] - The amount being transferred.
    ///
    pub fn move_tokens(
        &mut self,
        from: Address,
        to: Address,
        moved_token: Token,
        amount: TokenAmount,
    ) {
        self.deduct_from_token_balance(from, moved_token, amount);
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
    pub fn get_balance_for(&self, user: &Address) -> &TokenBalance {
        let token_balance = self.balances.get(user).unwrap_or(&EMPTY_BALANCE);
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
    pub fn get_mut_balance_for(&mut self, user: &Address) -> &mut TokenBalance {
        if !self.balances.contains_key(user) {
            self.balances.insert(*user, EMPTY_BALANCE);
        }
        self.balances.get_mut(user).unwrap()
    }

    /// Retrieves a pair of tokens with the `token_in_token_address` being the "token_in"-token
    /// and the remaining token being "token_out". <br>
    /// Requires that `token_in_token_address` matches the contract's pools.
    ///
    /// ### Parameters:
    ///
    /// * `token_in_token_address`: [`Token`] - The desired token to work with.
    ///
    /// # Returns
    /// The token_in/token_out-pair of tokens of type [`(Token, Token)`]
    pub fn deduce_tokens_in_out(&self, token_in_token_address: Address) -> TokensInOut {
        let token_in_a = self.token_a_address == token_in_token_address;
        let token_in_b = self.token_b_address == token_in_token_address;
        if !token_in_a && !token_in_b {
            panic!("token_in invalid token address")
        }
        if token_in_a {
            TokensInOut::A_IN_B_OUT
        } else {
            TokensInOut::B_IN_A_OUT
        }
    }
}

/// Tracks the from-to pairs for transfers, etc.
#[non_exhaustive]
pub struct TokensInOut {
    /// The input token.
    pub token_in: Token,
    /// The output token.
    pub token_out: Token,
}

impl TokensInOut {
    /// Token token_outs for when the user inputs [`Token::TokenA`], and outputs [`Token::TokenB`]
    pub const A_IN_B_OUT: Self = TokensInOut {
        token_in: Token::A,
        token_out: Token::B,
    };

    /// Token token_outs for when the user inputs [`Token::TokenB`], and outputs [`Token::TokenA`]
    pub const B_IN_A_OUT: Self = TokensInOut {
        token_in: Token::B,
        token_out: Token::A,
    };
}
