# Auction contract
The auction sells tokens of one type for another (can be the same token type). The contract works by escrowing bids as well as the tokens for sale. 

This is done through `transfer` calls to the token contracts with
callbacks ensuring that the transfers were successful.
If a bid is not the current highest bid the transferred bidding tokens can
be claimed during any phase.

The auction has a set `duration`. After this duration the auction no longer accepts bids and can
be executed by anyone. Once `execute` has been called the contract moves the tokens for sale
into the highest bidders claims and the highest bid into the contract owners claims.

In the bidding phase any account can call `bid` on the auction which makes a token `transfer`
from the bidder to the contract. Once the transfer is done the contract updates its
highest bidder accordingly.

The contract owner also has the ability to `cancel` the contract during the bidding phase.

If `cancel` is called the highest bid is taken out of escrow such that the highest bidder can
claim it again. The same is done for the tokens for sale which the contract owner
then can claim.