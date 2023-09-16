# Liquidity Swap
This contract is based on [UniSwap v1](https://hackmd.io/@HaydenAdams/HJ9jLsfTz?type=view)

The contracts exchanges (or swaps) between two types of tokens, 

with an the exchange rate as given by the `constant product formula: x * y = k`. 

We consider `x` to be the balance of token pool A and `y` to be the balance of token pool B and `k` to be their product. 

When performing a swap, a fee of 0.3% is applied, based on the input amount, which is deducted from the output of the swap. 

This effectively increases `k` after each swap.

In order to perform a swap, it is a prerequisite that the swapping user has already transferred
at least one of the tokens to the contract via a call to [`deposit`]. 

Additionally, some user (typically the creator of the contract) must have already deposited an amount of both token types and initialized both pools by a call to [`provide_initial_liquidity`]. 

A user may [`withdraw`] the resulting tokens of a swap (or simply his own deposited tokens)
to have the tokens transferred to his account, at any point.

Finally, a user may choose to become a liquidity provider (LP) of the contract
by providing an amount of pre-deposited tokens taken from the user's internal token balance.
This yields the LP a share of the contract's total liquidity, based on the ratio between the amount of provided liquidity and the contract's total liquidity at the time of providing. 

These shares are referred to as `liquidity tokens` which are minted upon becoming an LP and may later be burned to receive a proportionate share of the contract's liquidity. 

Since `k` increases between swaps, an LP stands to profit from burning their liquidity token after x amount of swaps has occurred.

The larger the shares an LP has, the larger the profit. 

However, as with all investing, an LP also risks losing profit if the market-clearing price of at least one of the tokens decreases to a point that exceeds the rewards gained from swap-fees.

Since liquidity tokens represent an equal share of both tokens, when providing liquidity it is enforced that the user provides an equivalent value of the opposite token to the tokens provided.

Because the relative price of the two tokens can only be changed through swapping,
divergences between the prices of the contract and the prices of similar external contracts create arbitrage opportunities.
This mechanism ensures that the contract's prices always trend toward the market-clearing price.