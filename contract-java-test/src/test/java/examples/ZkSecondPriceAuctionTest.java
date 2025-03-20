package examples;

import com.fasterxml.jackson.databind.JsonNode;
import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkSecondPriceAuction;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.FuzzyState;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.testenvironment.zk.node.EvmDataBuilder;
import com.partisiablockchain.language.testenvironment.zk.node.EvmEventLogBuilder;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.List;
import java.util.stream.IntStream;
import org.assertj.core.api.Assertions;
import org.bouncycastle.jcajce.provider.digest.Keccak;
import org.bouncycastle.util.encoders.Hex;

/** Test {@link ZkSecondPriceAuction}. */
public final class ZkSecondPriceAuctionTest extends JunitContractTest {

  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_second_price_auction.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_second_price_auction_runner"));

  private static final String ETH_CONTRACT_ADDRESS = "93b080860e7fb5745d11f081cd15556e1d72a15d";

  private List<BlockchainAddress> accounts;
  private BlockchainAddress owner;
  private BlockchainAddress auctionAddress;

  private ZkSecondPriceAuction auctionContract;

  /** Deploy auction contract. */
  @ContractTest
  void deploy() {
    accounts = IntStream.range(1, 10).mapToObj(blockchain::newAccount).toList();
    owner = blockchain.newAccount(999);

    auctionAddress =
        blockchain.deployZkContract(owner, CONTRACT_BYTES, ZkSecondPriceAuction.initialize());
    auctionContract = new ZkSecondPriceAuction(getStateClient(), auctionAddress);

    ZkSecondPriceAuction.ContractState state = auctionContract.getState();

    Assertions.assertThat(state.owner()).isEqualTo(owner);
    Assertions.assertThat(state.registeredBidders().size()).isEqualTo(0);
    Assertions.assertThat(state.auctionResult()).isNull();
    FuzzyState contractState = blockchain.getContractStateJson(auctionAddress);
    JsonNode attestations = contractState.getNode("/attestations");
    Assertions.assertThat(attestations).isEmpty();
  }

  /** Contract owner can add subscription to bidder registration events. */
  @ContractTest(previous = "deploy")
  void subscribeToBidderRegistration() {
    subscribeToBidderRegistrationEvents(owner, Hex.decode(ETH_CONTRACT_ADDRESS));

    FuzzyState contractState = blockchain.getContractStateJson(auctionAddress);
    JsonNode subscriptions = contractState.getNode("/externalEvents/subscriptions");
    Assertions.assertThat(subscriptions).hasSize(1);
    JsonNode subscription = subscriptions.get(0);
    JsonNode eventSignatureFilter =
        subscription.get("value").get("topics").get(0).get("topics").get(0).get("topic");
    Assertions.assertThat(eventSignatureFilter.toString().replace("\"", ""))
        .isEqualTo(Hex.toHexString(registrationCompleteEventSignature()));
  }

  /** Bidders can be registered via an external event. */
  @ContractTest(previous = "subscribeToBidderRegistration")
  void registerBidders() {
    registerAndAssertBidder(1, accounts.get(1), 1);
    registerAndAssertBidder(2, accounts.get(2), 2);
    registerAndAssertBidder(3, accounts.get(3), 3);
    registerAndAssertBidder(4, accounts.get(4), 4);
    registerAndAssertBidder(5, accounts.get(5), 5);
    registerAndAssertBidder(6, accounts.get(6), 6);
  }

  /** Registered users can bid on the contract. */
  @ContractTest(previous = "registerBidders")
  void placeBidsOnContract() {
    // Bids
    bidOnContract(accounts.get(1), 10);
    bidOnContract(accounts.get(2), 100000);
    bidOnContract(accounts.get(3), 13);
    bidOnContract(accounts.get(4), 20);
    bidOnContract(accounts.get(5), 22);
    bidOnContract(accounts.get(6), 256);
  }

  /** Contract owner can start the winner computation at any time. */
  @ContractTest(previous = "placeBidsOnContract")
  void startAuctionOnContract() {
    startAuction(owner);

    ZkSecondPriceAuction.ContractState state = auctionContract.getState();

    Assertions.assertThat(state.auctionResult().secondHighestBid()).isEqualTo(256);
    Assertions.assertThat(state.auctionResult().winner().address()).isEqualTo(accounts.get(2));
    Assertions.assertThat(state.auctionResult().winner().externalId()).isEqualTo(2);

    final var complexity = zkNodes.getComplexityOfLastComputation();
    Assertions.assertThat(complexity.numberOfRounds()).isEqualTo(364);
    Assertions.assertThat(complexity.multiplicationCount()).isEqualTo(1792);
  }

  /** The same user cannot be registered twice. */
  @ContractTest(previous = "registerBidders")
  void registerTwice() {
    Assertions.assertThatCode(() -> registerBidder(1, accounts.get(1), 6))
        .hasMessageContaining("Duplicate bidder address");
  }

  /** Users can only bid once. */
  @ContractTest(previous = "placeBidsOnContract")
  void bidTwice() {
    Assertions.assertThatCode(() -> bidOnContract(accounts.get(6), 1000))
        .hasMessageContaining("Each bidder is only allowed to place one bid");
  }

  /** Users must be registered to bid. */
  @ContractTest(previous = "deploy")
  void unregisteredBidder() {
    Assertions.assertThatCode(() -> bidOnContract(accounts.get(6), 1000))
        .hasMessageContaining("is not a registered bidder");
  }

  /** Contract owner can start the winner computation when nobody has bid. */
  @ContractTest(previous = "deploy")
  void nonOwnerCannotStartAuction() {
    Assertions.assertThatCode(() -> startAuction(accounts.get(6)))
        .hasMessageContaining("Only contract owner can start the auction");
  }

  /** Contract owner can start the winner computation when nobody has bid. */
  @ContractTest(previous = "deploy")
  void startAuctionWithZeroBids() {
    Assertions.assertThatCode(() -> startAuction(owner))
        .hasMessageContaining(
            "At least 3 bidders must have submitted bids for the auction to start");
  }

  /** Bidders cannot be registered after auction is done. */
  @ContractTest(previous = "startAuctionOnContract")
  void failToSetupBiddersAfterAuctionIsDone() {
    Assertions.assertThatCode(() -> registerBidder(32, owner, 7))
        .hasMessageContaining("Cannot register bidders after auction has begun");
  }

  /** Registered users can bid on the contract. */
  @ContractTest(previous = "startAuctionOnContract")
  void failToBidOnContractAfterAuctionIsDone() {
    Assertions.assertThatCode(() -> bidOnContract(accounts.get(6), 256))
        .hasMessageContaining("Cannot place bid after auction has begun");
  }

  /** Contract owner can start the winner computation at any time. */
  @ContractTest(previous = "startAuctionOnContract")
  void failtostartAuctionAfterAuctionIsDone() {
    Assertions.assertThatCode(() -> startAuction(owner))
        .hasMessageContaining("Cannot start auction after it has already begun");
  }

  private static byte[] registrationCompleteEventSignature() {
    Keccak.Digest256 keccak = new Keccak.Digest256();
    return keccak.digest("RegistrationComplete(int32,bytes21)".getBytes(StandardCharsets.UTF_8));
  }

  private void subscribeToBidderRegistrationEvents(BlockchainAddress sender, byte[] evmAddress) {
    byte[] subscribeRpc =
        ZkSecondPriceAuction.subscribeToBidderRegistration(evmAddress, BigInteger.ONE);
    blockchain.sendAction(sender, auctionAddress, subscribeRpc);
  }

  private void registerBidder(int bidderId, BlockchainAddress bidderAccount, long block) {
    EvmEventLogBuilder log =
        new EvmEventLogBuilder()
            // Workaround: block number is set since `TestZkNode` does not remember last block
            // between tests.
            .block(block)
            .from(ETH_CONTRACT_ADDRESS)
            .withTopic0(registrationCompleteEventSignature())
            .withData(new EvmDataBuilder().append(bidderId).append(bidderAccount));
    zkNodes.relayEvmEvent(log, auctionAddress);
  }

  private void registerAndAssertBidder(
      int bidderId, BlockchainAddress bidderAccount, int expectedBidderCount) {
    EvmEventLogBuilder log =
        new EvmEventLogBuilder()
            .from(ETH_CONTRACT_ADDRESS)
            .withTopic0(registrationCompleteEventSignature())
            .withData(new EvmDataBuilder().append(bidderId).append(bidderAccount));
    zkNodes.relayEvmEvent(log, auctionAddress);

    ZkSecondPriceAuction.ContractState state = auctionContract.getState();
    Assertions.assertThat(state.registeredBidders().size()).isEqualTo(expectedBidderCount);
    ZkSecondPriceAuction.RegisteredBidder registeredBidder =
        state.registeredBidders().get(bidderAccount);
    Assertions.assertThat(registeredBidder.externalId()).isEqualTo(bidderId);
    Assertions.assertThat(registeredBidder.haveAlreadyBid()).isEqualTo(false);
  }

  private void bidOnContract(BlockchainAddress bidder, int bidAmount) {
    CompactBitArray secretRpc =
        BitOutput.serializeBits(output -> output.writeUnsignedInt(bidAmount, 32));
    blockchain.sendSecretInput(auctionAddress, bidder, secretRpc, new byte[] {0x40});
  }

  private void startAuction(BlockchainAddress sender) {
    blockchain.sendAction(sender, auctionAddress, ZkSecondPriceAuction.startAuction());
  }
}
