# Zero Knowledge liquidity swap

 This is an example of a liquidity swap smart contract, with a simple form of frontrunning
 protection.

 This contract was based on the non-zk `liquidity-swap` variant.

 ## Swap Functionality

 The contract is a simplified implication of [UniSwap v2](https://docs.uniswap.org/protocol/V2/concepts/protocol-overview/how-uniswap-works)

 The contracts exchanges (or swaps) between two types of tokens,
 with an the exchange rate as given by the `constant product formula: x * y = k`.
 We consider `x` to be the balance of token pool A and `y` to be the balance of token pool B.
 `k` represents the invariant (`swap_constant`) that must be upheld between swaps.

 In order to perform a swap between the two desired tokens, the owner must first initialize
 both token pools, `initialize_pool_{a,b}`, by transferring an amount of tokens to both pools via a transfer call to
 the corresponding token contract. This will also initialize the (final) value of `k`.

 User's (including the owner) can then `deposit` tokens to the contract, which can be used to
 exchange to the opposite token. This is done by calling `swap`. `swap` will calculate the
 amount of tokens to convert of the incoming token to the opposite token, based on the above formula.
 A user may then `withdraw` the resulting tokens of the swap (or simply his own deposited tokens).

 Finally, the owner of the contract may close the pools, `close_pools`, by transferring both token pools to his own account,
 effectively closing the contract. Only valid withdrawals are allowed in the closed state.

 Both `deposit` and `withdraw` makes use of `transfer` calls to the token contract, which
 are ensured to be successful via callbacks.

 Because the relative price of the two tokens can only be changed through swapping,
 divergences between the prices of the current contract and the prices of similar external contracts create arbitrage opportunities.
 This mechanism ensures that the contract's prices always trend toward the market-clearing price.

 ## Frontrunning protection through secret-shared input

 The contract uses basic ZK functionality in order to commit to swap direction and amount
 before these are visible to the block producer. This prevents basic frontrunning, as the
 information needed to front run is no longer available at the point in time where the
 frontrunning is possible. It is still possible for a block producer to perform statistical
 analysis to determine which swap direction is likely, though this is a significantly higher
 barrier to entry.

 ## Differences to non-zk swap contract

 The non-zk swap contract is capable of being called very quickly, whereas this contract needs
 to perform some slow zk operations:

 - Secret sharing inputs
 - Small computation
 - Opening secret shares

 Due to the slowness of this process, the contract maintains a queue of swaps, to guarentee
 fairness.

 ## `perform_calls` feature

 Disabling the `perform_calls` feature allows for easier testing of the contract, as it will
 avoid calling to other contracts. In effect, this will allow anyone to pretend like they have
 any amount of tokens, as the contract is unable to verify with the token contracts.
