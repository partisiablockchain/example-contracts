# Zero Knowledge: Second Price Auction

**This version of the contract is a pure PBC implementation. Also see the
[version that acts as a layer 2 second for Ethereum](../zk-as-a-service-second-price-auction).**

Simple Second Price Auction (also known as a [Vickrey auction](https://en.wikipedia.org/wiki/Vickrey_auction)) Contract.
Second price auctions is a common form of auction, where each party places a bid, and the
winner is the party who places the highest bid. However, the winner only pays the amount of the
second highest bid. ZK implementations of such auctions facilities the possibility of such
auctions without revealing the incoming bids - making the auction fair.

One of the great advantages of PBC over other blockchains is that zero knowledge computations can be performed on the
network parallel to the public transactions on the blockchain. The second price auction takes as inputs the bids from
the registered participants. The bids are delivered encrypted and secret-shared to the ZK nodes allocated to the contract.
When the computation is initiated by the contract owner, the zero knowledge computation nodes reads the collected input
and then create a bit vector consisting of prices and the ordering number. The list of bit vectors is now sorted in MPC.
The winner is the first entry (the bidder with the highest price-bid),
the price is determined by the size of the second-highest bid.

### Usage

1. Initialization on the blockchain.
2. Receival of secret bids, using zero-knowledge protocols.
3. Once enough bids have been received, the owner of the contract can initialize the auction.
4. The ZK computation computes the winning bid in a secure manner.
5. Once the ZK computation concludes, the winning bid will be published and the winner will be
   stored in the state, together with their bid.
