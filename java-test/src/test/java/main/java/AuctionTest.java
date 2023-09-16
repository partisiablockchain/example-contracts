package main.java;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.AuctionContract;
import com.partisiablockchain.language.abicodegen.TokenContract;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import java.math.BigInteger;
import java.nio.file.Path;

/** Test suite for the auction smart contract. */
public final class AuctionTest extends JunitContractTest {

  private static final ContractBytes AUCTION_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/auction_contract.wasm"),
          Path.of("../target/wasm32-unknown-unknown/release/auction_contract.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/auction_contract_runner"));
  private static final ContractBytes TOKEN_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/token_contract.wasm"),
          Path.of("../target/wasm32-unknown-unknown/release/token_contract.abi"));

  private static final int DOGE_SUPPLY = 100000000;
  private static final int BITCOIN_SUPPLY = 10000;
  private BlockchainAddress ownerDoge;
  private BlockchainAddress ownerBitcoin;
  private BlockchainAddress auctionOwner;
  private BlockchainAddress bidder1;
  private BlockchainAddress bidder2;
  private BlockchainAddress bidder3;
  private BlockchainAddress bitcoin;
  private BlockchainAddress doge;
  private BlockchainAddress auction;

  private static final byte CREATION = 0;
  private static final byte BIDDING = 1;
  private static final byte ENDED = 2;
  private static final byte CANCELLED = 3;

  /**
   * Setup for other tests. Instantiates the accounts of all the contract owners deploys the
   * contracts and transfers and approves tokens for the bidders and the auction contract.
   */
  @ContractTest
  void setup() {
    // instantiate accounts
    ownerDoge = blockchain.newAccount(2);
    ownerBitcoin = blockchain.newAccount(3);
    auctionOwner = blockchain.newAccount(4);
    bidder1 = blockchain.newAccount(5);
    bidder2 = blockchain.newAccount(6);
    bidder3 = blockchain.newAccount(7);

    // deploy token contracts
    byte[] dogeInitRpc =
        TokenContract.initialize("Doge Coin", "DOGE", (byte) 18, BigInteger.valueOf(DOGE_SUPPLY));
    doge = blockchain.deployContract(ownerDoge, TOKEN_CONTRACT_BYTES, dogeInitRpc);
    byte[] bitcoinInitRpc =
        TokenContract.initialize("Bitcoin", "BTC", (byte) 18, BigInteger.valueOf(BITCOIN_SUPPLY));
    bitcoin = blockchain.deployContract(ownerBitcoin, TOKEN_CONTRACT_BYTES, bitcoinInitRpc);
    // transfer funds to the bidders and the contract owner
    byte[] transferOne = TokenContract.transfer(bidder1, BigInteger.valueOf(500));
    byte[] transferTwo = TokenContract.transfer(bidder2, BigInteger.valueOf(1000));
    byte[] transferThree = TokenContract.transfer(bidder3, BigInteger.valueOf(1500));
    byte[] transferFour = TokenContract.transfer(auctionOwner, BigInteger.valueOf(1500));
    blockchain.sendAction(ownerDoge, doge, transferOne);
    blockchain.sendAction(ownerDoge, doge, transferTwo);
    blockchain.sendAction(ownerDoge, doge, transferThree);
    blockchain.sendAction(ownerBitcoin, bitcoin, transferFour);

    // assert that the transfers were successful
    TokenContract.TokenState dogeState =
        TokenContract.TokenState.deserialize(blockchain.getContractState(doge));
    assertThat(dogeState.balances().get(bidder1)).isEqualTo(500);
    assertThat(dogeState.balances().get(bidder2)).isEqualTo(1000);
    assertThat(dogeState.balances().get(bidder3)).isEqualTo(1500);

    TokenContract.TokenState bitcoinState =
        TokenContract.TokenState.deserialize(blockchain.getContractState(bitcoin));
    assertThat(bitcoinState.balances().get(auctionOwner)).isEqualTo(1500);

    // deploy the auction contract
    byte[] auctionInitRpc =
        AuctionContract.initialize(
            BigInteger.valueOf(50),
            bitcoin,
            doge,
            BigInteger.valueOf(20),
            BigInteger.valueOf(5),
            2);

    auction = blockchain.deployContract(auctionOwner, AUCTION_CONTRACT_BYTES, auctionInitRpc);

    bitcoinState = TokenContract.TokenState.deserialize(blockchain.getContractState(bitcoin));

    assertThat(bitcoinState.balances().get(auctionOwner)).isEqualTo(1500);
    assertThat(bitcoinState.balances().get(auction)).isNull();

    // approve auction contract to make transactions on behalf of owner
    byte[] approveRpc = TokenContract.approve(auction, BigInteger.valueOf(50));

    blockchain.sendAction(auctionOwner, bitcoin, approveRpc);

    // approve auction contract to make transactions on behalf of bidders
    byte[] approveForAuctionBidderOneRpc = TokenContract.approve(auction, BigInteger.valueOf(500));
    byte[] approveForAuctionBidderTwoRpc = TokenContract.approve(auction, BigInteger.valueOf(1000));
    byte[] approveForAuctionBidderThreeRpc =
        TokenContract.approve(auction, BigInteger.valueOf(1500));
    blockchain.sendAction(bidder1, doge, approveForAuctionBidderOneRpc);
    blockchain.sendAction(bidder2, doge, approveForAuctionBidderTwoRpc);
    blockchain.sendAction(bidder3, doge, approveForAuctionBidderThreeRpc);

    assertThat(allowance(bidder1, auction)).isEqualTo(BigInteger.valueOf(500));
    assertThat(allowance(bidder2, auction)).isEqualTo(BigInteger.valueOf(1000));
    assertThat(allowance(bidder3, auction)).isEqualTo(BigInteger.valueOf(1500));

    // start the auction
    byte[] startRpc = AuctionContract.start();
    blockchain.sendAction(auctionOwner, auction, startRpc);

    bitcoinState = TokenContract.TokenState.deserialize(blockchain.getContractState(bitcoin));
    assertThat(bitcoinState.balances().get(auction)).isEqualTo(BigInteger.valueOf(50));

    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);
  }

  private BigInteger allowance(final BlockchainAddress owner, final BlockchainAddress spender) {
    final TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(doge));
    return state.allowed().get(owner).get(spender);
  }

  /** The highest bid is registered as highest bid. */
  @ContractTest(previous = "setup")
  void makeBid() {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);

    byte[] bidForFifty = AuctionContract.bid(BigInteger.valueOf(50));
    blockchain.sendAction(bidder1, auction, bidForFifty);

    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    assertThat(auctionState.highestBidder().bidder()).isEqualTo(bidder1);
    assertThat(auctionState.highestBidder().amount()).isEqualTo(BigInteger.valueOf(50));
  }

  /** A bid smaller than the auction's reserve price is registered in the Claim map immediately. */
  @ContractTest(previous = "setup")
  void tooSmallBid() {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);

    byte[] bidForOne = AuctionContract.bid(BigInteger.valueOf(1));
    // Assert that the bid is lower than the reserve price
    assertThat(auctionState.reservePrice().intValue() > 1).isTrue();

    blockchain.sendAction(bidder1, auction, bidForOne);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    // Assert that the bid can be claimed immediately.
    assertThat(auctionState.claimMap().size()).isEqualTo(1);
    assertThat(auctionState.claimMap().get(bidder1).tokensForBidding()).isEqualTo(1);
  }

  /** When the auction owner calls cancel in the BIDDING phase, the auction is cancelled. */
  @ContractTest(previous = "setup")
  void cancelAuction() {
    byte[] cancelRpc = AuctionContract.cancel();
    blockchain.sendAction(auctionOwner, auction, cancelRpc);
    AuctionContract.AuctionContractState auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(CANCELLED);
  }

  /** The winner can claim the auction prize and the auction owner can claim the highest bid. */
  @ContractTest(previous = "setup")
  void bidAndClaim() {
    // make bid
    byte[] bidForThirty = AuctionContract.bid(BigInteger.valueOf(30));
    blockchain.sendAction(bidder3, auction, bidForThirty);
    AuctionContract.AuctionContractState auctionState;

    // pass time
    long auctionEndTime = 3 * 60 * 60 * 1000;
    blockchain.waitForBlockProductionTime(auctionEndTime);

    // execute auction
    byte[] executeRpc = AuctionContract.execute();
    blockchain.sendAction(auctionOwner, auction, executeRpc);

    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(ENDED); // status should be ENDED
    assertThat(auctionState.claimMap().size())
        .isEqualTo(2); // one claim for bidder and one for seller
    assertThat(auctionState.claimMap().get(bidder3).tokensForSale())
        .isEqualTo(BigInteger.valueOf(50)); // bidder's claim should be equal to amount for sale
    assertThat(auctionState.claimMap().get(auctionOwner).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(30)); // seller's claim should be equal to bid

    // make claims
    byte[] claimRpc = AuctionContract.claim();
    blockchain.sendAction(bidder3, auction, claimRpc);
    blockchain.sendAction(auctionOwner, auction, claimRpc);

    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    assertThat(auctionState.claimMap().size())
        .isEqualTo(2); // size of claim map should remain the same
    assertThat(auctionState.claimMap().get(bidder3).tokensForSale())
        .isEqualTo(BigInteger.valueOf(0)); // tokens should now be claimed
    assertThat(auctionState.claimMap().get(auctionOwner).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(0)); // tokens should now be claimed
  }

  /**
   * At the end of an auction, the highest bidder can claim the prize, the other bidders can claim
   * their bids, and the owner can claim the highest bid.
   */
  @ContractTest(previous = "setup")
  void biddersAndWinnersCanClaimBidsAndPrize() {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);

    // make bids
    byte[] bidForTwenty = AuctionContract.bid(BigInteger.valueOf(20));
    blockchain.sendAction(bidder1, auction, bidForTwenty);
    byte[] bidForThirty = AuctionContract.bid(BigInteger.valueOf(30));
    blockchain.sendAction(bidder2, auction, bidForThirty);

    // pass time
    long auctionEndTime = 3 * 60 * 60 * 1000;
    blockchain.waitForBlockProductionTime(auctionEndTime);

    // execute auction
    byte[] executeRpc = AuctionContract.execute();
    blockchain.sendAction(auctionOwner, auction, executeRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    // status should be ENDED
    assertThat(auctionState.status()).isEqualTo(ENDED);
    assertThat(auctionState.claimMap().size()).isEqualTo(3);
    // bidder who didn't win should be able to claim their bid
    assertThat(auctionState.claimMap().get(bidder1).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(20));
    // the highest bidder (auction winner) should be able to claim the prize of the auction
    assertThat(auctionState.claimMap().get(bidder2).tokensForSale())
        .isEqualTo(BigInteger.valueOf(50));
    // the auction owner should be able to claim the highest bid
    assertThat(auctionState.claimMap().get(auctionOwner).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(30));

    byte[] claimRpc = AuctionContract.claim();
    blockchain.sendAction(bidder1, auction, claimRpc);
    blockchain.sendAction(bidder2, auction, claimRpc);
    blockchain.sendAction(auctionOwner, auction, claimRpc);

    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    // Assert that the claims have been claimed
    assertThat(auctionState.claimMap().get(bidder1).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(0));
    assertThat(auctionState.claimMap().get(bidder2).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(0));
    assertThat(auctionState.claimMap().get(auctionOwner).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(0));
  }

  /**
   * A bidder bids an amount which is lower than the highest bid. Should be able to claim it
   * afterward.
   */
  @ContractTest(previous = "setup")
  void bidsLowerThanHighestBid() {
    bidTwiceAndMakeAssertions(40, 30);
  }

  /** The first of two bids with the same amount bid is registered as the highest. */
  @ContractTest(previous = "setup")
  void bidTheHighestBid() {
    bidTwiceAndMakeAssertions(30, 30);
  }

  /**
   * A bidder bids two different bids. The highest wins, and the lowest can be claimed at the end of
   * the auction.
   */
  @ContractTest(previous = "setup")
  void bidTwoDifferentBids() {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);

    byte[] highBid = AuctionContract.bid(BigInteger.valueOf(30));
    blockchain.sendAction(bidder1, auction, highBid);

    // Another bidder bids the current highest bid
    byte[] lowBid = AuctionContract.bid(BigInteger.valueOf(20));
    blockchain.sendAction(bidder1, auction, lowBid);

    // pass time
    long auctionEndTime = 3 * 60 * 60 * 1000;
    blockchain.waitForBlockProductionTime(auctionEndTime);

    // execute auction
    byte[] executeRpc = AuctionContract.execute();
    blockchain.sendAction(auctionOwner, auction, executeRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    assertThat(auctionState.claimMap().size()).isEqualTo(2);
    // The bidder should be able to claim the lower one of their bids.
    assertThat(auctionState.claimMap().get(bidder1).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(20));
    // The bidder should be able to claim the auction prize.
    assertThat(auctionState.claimMap().get(bidder1).tokensForSale())
        .isEqualTo(BigInteger.valueOf(50));
    // The auction owner should be able to claim the highest of the two bids.
    assertThat(auctionState.claimMap().get(auctionOwner).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(30));
  }

  /** Auction winner attempts to claim twice. Second claim has no effect. */
  @ContractTest(previous = "setup")
  void winnerClaimsTwice() {
    byte[] bidRpc = AuctionContract.bid(BigInteger.valueOf(30));
    blockchain.sendAction(bidder3, auction, bidRpc);

    // pass time past auction end
    long auctionEndTime = 3 * 60 * 60 * 1000;
    blockchain.waitForBlockProductionTime(auctionEndTime);

    byte[] executeRpc = AuctionContract.execute();
    blockchain.sendAction(auctionOwner, auction, executeRpc);
    AuctionContract.AuctionContractState auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.claimMap().size()).isEqualTo(2);

    byte[] claimRpc = AuctionContract.claim();
    blockchain.sendAction(bidder3, auction, claimRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    assertThat(auctionState.claimMap().get(bidder3).tokensForSale()).isEqualTo(0);
    assertThat(auctionState.claimMap().get(bidder3).tokensForBidding()).isEqualTo(0);
    blockchain.sendAction(bidder3, auction, claimRpc); // claiming twice should do nothing
    assertThat(auctionState.claimMap().get(bidder3).tokensForSale()).isEqualTo(0);
    assertThat(auctionState.claimMap().get(bidder3).tokensForBidding()).isEqualTo(0);
  }

  /** Non-owner cannot start the auction. */
  @ContractTest(previous = "setup")
  void startCalledByNonOwner() {
    byte[] startRpc = AuctionContract.start();
    assertThatThrownBy(() -> blockchain.sendAction(blockchain.newAccount(15), auction, startRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Start can only be called by the creator of the contract");
  }

  /** Non-owner cannot cancel an auction. */
  @ContractTest(previous = "setup")
  void cancelAuctionNonOwner() {
    byte[] cancelRpc = AuctionContract.cancel();
    assertThatThrownBy(() -> blockchain.sendAction(blockchain.newAccount(25), auction, cancelRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only the contract owner can cancel the auction");
  }

  /** Cancelling the auction when the phase is not BIDDING is not possible. */
  @ContractTest(previous = "setup")
  void cancelAuctionStatusNotBidding() {
    byte[] cancelRpc = AuctionContract.cancel();
    blockchain.sendAction(auctionOwner, auction, cancelRpc);
    assertThatThrownBy(() -> blockchain.sendAction(auctionOwner, auction, cancelRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Tried to cancel the auction when the status isn't Bidding");
  }

  /** Execution of auction cannot happen before the deadline. */
  @ContractTest(previous = "setup")
  void executeBeforeEndTime() {
    byte[] executeRpc = AuctionContract.execute();
    assertThatThrownBy(() -> blockchain.sendAction(auctionOwner, auction, executeRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Tried to execute the auction before auction end block time");
  }

  /**
   * User who has not bid does not appear on the claim map, and nothing happens if they try to
   * claim.
   */
  @ContractTest(previous = "setup")
  void claimFromUserWithNoClaim() {
    BlockchainAddress account = blockchain.newAccount(30);
    byte[] claimRpc = AuctionContract.claim();
    blockchain.sendAction(account, auction, claimRpc);
    var auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.claimMap().size()).isEqualTo(0);
  }

  /** Auction owner attempts to claim payment before end time, has no effect. */
  @ContractTest(previous = "setup")
  void ownerClaimBeforeAuctionEnded() {
    AuctionContract.AuctionContractState auctionState;
    byte[] bidTwentyRpc = AuctionContract.bid(BigInteger.valueOf(20));
    byte[] bidThirtyRpc = AuctionContract.bid(BigInteger.valueOf(30));

    blockchain.sendAction(bidder2, auction, bidTwentyRpc);
    blockchain.sendAction(bidder3, auction, bidThirtyRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.claimMap().get(auctionOwner).tokensForSale()).isEqualTo(0);

    byte[] claimRpc = AuctionContract.claim();
    blockchain.sendAction(auctionOwner, auction, claimRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.claimMap().get(auctionOwner).tokensForSale()).isEqualTo(0);
    assertThat(auctionState.highestBidder().bidder()).isEqualTo(bidder3);
  }

  /** Execute an auction before it was started is not possible. */
  @ContractTest(previous = "setup")
  void executeWhenStatusNotBidding() {

    byte[] bidRpc = AuctionContract.bid(BigInteger.TEN);
    blockchain.sendAction(bidder3, auction, bidRpc);

    long auctionEndTime = 3 * 60 * 60 * 1000;
    blockchain.waitForBlockProductionTime(auctionEndTime);
    byte[] executeRpc = AuctionContract.execute();
    blockchain.sendAction(
        auctionOwner, auction, executeRpc); // execute first to change status from Bidding
    AuctionContract.AuctionContractState auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(ENDED);
    assertThatThrownBy(() -> blockchain.sendAction(auctionOwner, auction, executeRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Tried to execute the auction when the status isn't Bidding");
  }

  /** An auction that has already ended, cannot be cancelled. */
  @ContractTest(previous = "setup")
  void cancelAuctionAfterEnd() {
    long auctionEndTime = 3 * 60 * 60 * 1000;
    blockchain.waitForBlockProductionTime(auctionEndTime);
    AuctionContract.AuctionContractState auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);

    byte[] cancelRpc = AuctionContract.cancel();
    assertThatThrownBy(() -> blockchain.sendAction(auctionOwner, auction, cancelRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Tried to cancel the auction after auction end block time");
  }

  /** Highest bidder attempts to claim before end time, has no effect. */
  @ContractTest(previous = "setup")
  void highestBidderClaimBeforeAuctionEnded() {
    AuctionContract.AuctionContractState auctionState;
    byte[] bidTwentyRpc = AuctionContract.bid(BigInteger.valueOf(20));
    byte[] bidThirtyRpc = AuctionContract.bid(BigInteger.valueOf(30));

    blockchain.sendAction(bidder2, auction, bidTwentyRpc);
    blockchain.sendAction(bidder3, auction, bidThirtyRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.claimMap().get(bidder3)).isNull();

    byte[] claimRpc = AuctionContract.claim();
    blockchain.sendAction(bidder3, auction, claimRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.claimMap().get(bidder3)).isNull();
    assertThat(auctionState.highestBidder().bidder()).isEqualTo(bidder3);
  }

  /** An auction cannot be deployed with a non-public token Address for bidding. */
  @ContractTest(previous = "setup")
  void nonPublicBidToken() {
    byte[] auctionInitRpcBidIllegal =
        AuctionContract.initialize(
            BigInteger.valueOf(50),
            doge,
            blockchain.newAccount(12),
            BigInteger.valueOf(20),
            BigInteger.valueOf(5),
            2);

    assertThatThrownBy(
            () ->
                blockchain.deployContract(
                    auctionOwner, AUCTION_CONTRACT_BYTES, auctionInitRpcBidIllegal))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Tried to create a contract buying a non publicContract token");
  }

  /** An auction cannot be selling a non-public token. */
  @ContractTest(previous = "setup")
  void nonPublicSaleAuction() {
    byte[] auctionInitRpcSaleIllegal =
        AuctionContract.initialize(
            BigInteger.valueOf(50),
            blockchain.newAccount(10),
            doge,
            BigInteger.valueOf(20),
            BigInteger.valueOf(5),
            2);

    assertThatThrownBy(
            () ->
                blockchain.deployContract(
                    auctionOwner, AUCTION_CONTRACT_BYTES, auctionInitRpcSaleIllegal))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Tried to create a contract selling a non publicContract token");
  }

  /** The start of an auction is not allowed unless it is in the CREATION phase. */
  @ContractTest(previous = "setup")
  void startCalledNotCreationStatus() {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isNotEqualTo(CREATION);
    byte[] startRpc = AuctionContract.start();
    assertThatThrownBy(() -> blockchain.sendAction(auctionOwner, auction, startRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Start should only be called while setting up the contract");
  }

  /** Bidder makes a bid before approving auction contract to transfer the funds, bid fails. */
  @ContractTest(previous = "setup")
  void bidTokenNotApproved() {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);
    byte[] transferThousand = TokenContract.transfer(bidder2, BigInteger.valueOf(1000));
    BlockchainAddress bidderNotApproved = blockchain.newAccount(17);
    blockchain.sendAction(ownerDoge, doge, transferThousand); // transfer funds to bidder

    byte[] bidForTen = AuctionContract.bid(BigInteger.valueOf(10));
    // bidder tries to bid before being approved to transfer the token
    assertThatThrownBy(() -> blockchain.sendAction(bidderNotApproved, auction, bidForTen))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Insufficient allowance for transfer_from: 0/10");
  }

  /** Bidder cannot bid more than they allowed the auction to transfer. */
  @ContractTest(previous = "setup")
  void bidTokenNotEnoughFunds() {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);

    byte[] bidForTenThousand = AuctionContract.bid(BigInteger.valueOf(10_000));
    assertThatThrownBy(() -> blockchain.sendAction(bidder1, auction, bidForTenThousand))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Insufficient allowance for transfer_from: 500/10000");
  }

  /**
   * Auction owner cannot start an auction selling more tokens than they hold of the token for sale.
   */
  @ContractTest(previous = "setup")
  void startAuctionOwnerNotEnoughFunds() {
    byte[] auctionInitRpc =
        AuctionContract.initialize(
            BigInteger.valueOf(100),
            bitcoin,
            doge,
            BigInteger.valueOf(20),
            BigInteger.valueOf(5),
            2);

    auction = blockchain.deployContract(auctionOwner, AUCTION_CONTRACT_BYTES, auctionInitRpc);

    TokenContract.TokenState bitcoinState;
    bitcoinState = TokenContract.TokenState.deserialize(blockchain.getContractState(bitcoin));

    assertThat(bitcoinState.balances().get(auction)).isNull();

    byte[] approveRpc = TokenContract.approve(auction, BigInteger.valueOf(50)); // only 50 approved

    AuctionContract.AuctionContractState auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    assertThat(auctionState.status()).isEqualTo(CREATION);

    blockchain.sendAction(auctionOwner, bitcoin, approveRpc);

    byte[] startRpc = AuctionContract.start();
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    auctionOwner, auction, startRpc) // try to start auction with 100 for sale
            )
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Insufficient allowance for transfer_from: 50/100");

    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    assertThat(auctionState.status()).isEqualTo(CREATION);
  }

  void bidTwiceAndMakeAssertions(long bidOne, long bidTwo) {
    AuctionContract.AuctionContractState auctionState;
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));
    assertThat(auctionState.status()).isEqualTo(BIDDING);

    byte[] bidder1Bid = AuctionContract.bid(BigInteger.valueOf(bidOne));
    blockchain.sendAction(bidder1, auction, bidder1Bid);

    // bid a lower amount than the highest bid
    byte[] bidder2Bid = AuctionContract.bid(BigInteger.valueOf(bidTwo));
    blockchain.sendAction(bidder2, auction, bidder2Bid);

    // pass time
    long auctionEndTime = 3 * 60 * 60 * 1000;
    blockchain.waitForBlockProductionTime(auctionEndTime);

    // execute auction
    byte[] executeRpc = AuctionContract.execute();
    blockchain.sendAction(auctionOwner, auction, executeRpc);
    auctionState =
        AuctionContract.AuctionContractState.deserialize(blockchain.getContractState(auction));

    assertThat(auctionState.claimMap().size()).isEqualTo(3);
    assertThat(auctionState.claimMap().get(bidder2).tokensForBidding())
        .isEqualTo(BigInteger.valueOf(30));
    assertThat(auctionState.claimMap().get(bidder1).tokensForSale())
        .isEqualTo(BigInteger.valueOf(50));
  }
}
