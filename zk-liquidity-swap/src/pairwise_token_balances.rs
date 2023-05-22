//! Contains a generalized token balance data structure.
use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Type used to represent token amounts.
pub type TokenAmount = u128;

/// Enum for token types
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, ReadWriteRPC, Debug, CreateTypeSpec)]
pub enum Token {
    /// The value representing token A.
    #[discriminant(0)]
    TokenA {},
    /// The value representing token B.
    #[discriminant(1)]
    TokenB {},
}

impl Token {
    pub const A: Token = Token::TokenA {};
    pub const B: Token = Token::TokenB {};
}

/// Holds user balances for the two tokens.
/// Keeps track of how much of a given token a user can withdraw.
#[derive(ReadWriteState, CreateTypeSpec, Clone, PartialEq, Eq, Debug)]
pub struct Balance {
    /// The amount of token A that a user owns, managed by the contract.
    pool_a_balance: TokenAmount,
    /// The amount of token B that a user owns, managed by the contract.
    pool_b_balance: TokenAmount,
}

/// Empty balance.
const EMPTY_BALANCE: Balance = Balance {
    pool_a_balance: 0,
    pool_b_balance: 0,
};

impl Balance {
    /// The balance of the given token.
    pub fn for_token(&self, token: Token) -> TokenAmount {
        if token == Token::A {
            self.pool_a_balance
        } else {
            self.pool_b_balance
        }
    }

    fn get_mut_balance_for(&mut self, token: Token) -> &mut TokenAmount {
        if token == Token::A {
            &mut self.pool_a_balance
        } else {
            &mut self.pool_b_balance
        }
    }

    /// True if both balances are zero.
    pub fn is_empty(&self) -> bool {
        self.pool_a_balance == 0 && self.pool_b_balance == 0
    }
}

/// Generalized token balance structure.
#[derive(ReadWriteState, CreateTypeSpec, Clone, PartialEq, Eq, Debug)]
pub struct PairwiseTokenBalances {
    token_a: Address,
    token_b: Address,
    balances: SortedVecMap<Address, Balance>,
}

impl PairwiseTokenBalances {
    pub fn new(token_a: Address, token_b: Address) -> Self {
        Self {
            token_a,
            token_b,
            balances: SortedVecMap::new(),
        }
    }

    /// Adds tokens to the `balances` map of the contract.
    /// If the user isn't already present, creates an entry with an empty Balance.
    ///
    /// ### Parameters:
    ///
    /// * `user`: The address of the user.
    ///
    /// * `token`: The token to add to.
    ///
    /// * `amount`: The amount to add.
    ///
    pub fn deposit_to_user_balance(&mut self, user: Address, token: Token, amount: TokenAmount) {
        if !self.balances.contains_key(&user) {
            self.balances.insert(
                user,
                Balance {
                    pool_a_balance: 0,
                    pool_b_balance: 0,
                },
            );
        }
        let user_balance = self.balances.get_mut(&user).unwrap();

        *user_balance.get_mut_balance_for(token) += amount;
    }

    /// Subtracts tokens from the `balances` map of the contract.
    /// Requires that the user already has an entry and that the subtraction yields a non-negative value.
    ///
    /// ### Parameters:
    ///
    /// * `user`: The address of the user.
    ///
    /// * `token`: The token to subtract from.
    ///
    /// * `amount`: The amount to subtract.
    ///
    pub fn withdraw_from_user_balance(
        &mut self,
        user: &Address,
        token: Token,
        amount: TokenAmount,
    ) -> Result<(), String> {
        let user_balance = self.balances.get_mut(user).ok_or("Need existing balance")?;

        let token_balance = user_balance.get_mut_balance_for(token);
        let new_token_balance = token_balance.checked_sub(amount).ok_or_else(|| {
            format!("Insufficient funds {token_balance}, expected at least {amount}",)
        })?;

        *token_balance = new_token_balance;

        if user_balance.is_empty() {
            self.balances.remove(user);
        }

        Ok(())
    }

    pub fn transfer_from_to(
        &mut self,
        from: &Address,
        to: Address,
        token: Token,
        amount: TokenAmount,
    ) -> Result<(), String> {
        self.withdraw_from_user_balance(from, token, amount)?;
        self.deposit_to_user_balance(to, token, amount);
        Ok(())
    }

    /// Determines the amount of tokens a user possesses.
    ///
    /// ### Parameters:
    ///
    /// * `user`: The address of the user.
    ///
    pub fn get_balance(&self, user: &Address) -> &Balance {
        self.balances.get(user).unwrap_or(&EMPTY_BALANCE)
    }

    /// Retrieves a pair of tokens with the `input_token_address` being the "from"-token
    /// and the remaining token being "to".
    /// Requires that `input_token_address` matches the contract's pools.
    ///
    /// ### Parameters:
    ///
    /// * `input_token_address`: The token matching the desired pool.
    ///
    /// # Returns
    ///
    /// The from/to-pair of tokens of type [`(Token, Token)`]
    pub fn deduce_from_to_tokens(&self, input_token_address: &Address) -> Option<(Token, Token)> {
        let is_from_a = self.token_a == *input_token_address;
        let is_from_b = self.token_b == *input_token_address;
        if !is_from_a && !is_from_b {
            return None;
        }
        Some(self.deduce_from_to_tokens_b(is_from_a))
    }

    pub fn deduce_from_to_tokens_b(&self, is_from_a: bool) -> (Token, Token) {
        if is_from_a {
            (Token::A, Token::B)
        } else {
            (Token::B, Token::A)
        }
    }
}
