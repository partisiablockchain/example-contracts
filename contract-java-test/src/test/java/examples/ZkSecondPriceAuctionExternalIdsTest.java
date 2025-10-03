package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkSecondPriceAuctionExternalIds;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import java.util.List;
import java.util.stream.IntStream;
import org.assertj.core.api.Assertions;

/** Test {@link ZkSecondPriceAuctionExternalIds}. */
public final class ZkSecondPriceAuctionExternalIdsTest extends JunitContractTest {

  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of(
              "../rust/target/wasm32-unknown-unknown/release/"
                  + "zk_second_price_auction_external_ids.pbc"),
          Path.of(
              "../rust/target/wasm32-unknown-unknown/release/"
                  + "zk_second_price_auction_external_ids_runner"));

  private List<BlockchainAddress> accounts;
  private BlockchainAddress owner;

  private BlockchainAddress auctionContractAddress;
  private ZkSecondPriceAuctionExternalIds auctionContract;

  /** Deploy auction contract. */
  @ContractTest
  void deploy() {
    accounts = IntStream.range(1, 10).mapToObj(blockchain::newAccount).toList();
    owner = blockchain.newAccount(999);

    auctionContractAddress =
        blockchain.deployZkContract(
            owner, CONTRACT_BYTES, ZkSecondPriceAuctionExternalIds.initialize());
    auctionContract = new ZkSecondPriceAuctionExternalIds(getStateClient(), auctionContractAddress);

    ZkSecondPriceAuctionExternalIds.ContractState state = auctionContract.getState().openState();

    Assertions.assertThat(state).isNotNull();
  }

  /** Contract owner can register which users can bid on the contract. */
  @ContractTest(previous = "deploy")
  void setupBidders() {
    registerBidders(
        owner,
        List.of(
            new ZkSecondPriceAuctionExternalIds.AddressAndExternalId(
                accounts.get(1), externalId(1)),
            new ZkSecondPriceAuctionExternalIds.AddressAndExternalId(
                accounts.get(2), externalId(2)),
            new ZkSecondPriceAuctionExternalIds.AddressAndExternalId(
                accounts.get(3), externalId(3)),
            new ZkSecondPriceAuctionExternalIds.AddressAndExternalId(
                accounts.get(4), externalId(4)),
            new ZkSecondPriceAuctionExternalIds.AddressAndExternalId(
                accounts.get(5), externalId(5)),
            new ZkSecondPriceAuctionExternalIds.AddressAndExternalId(
                accounts.get(6), externalId(6))));
  }

  private static ZkSecondPriceAuctionExternalIds.ExternalId externalId(int b) {
    return new ZkSecondPriceAuctionExternalIds.ExternalId(new byte[] {0, (byte) b});
  }

  /** Registered users can bid on the contract. */
  @ContractTest(previous = "setupBidders")
  void placeBidsOnContract() {
    // Bids
    bidOnAuction(accounts.get(1), 10);
    bidOnAuction(accounts.get(2), 100000);
    bidOnAuction(accounts.get(3), 13);
    bidOnAuction(accounts.get(4), 20);
    bidOnAuction(accounts.get(5), 22);
    bidOnAuction(accounts.get(6), 256);
  }

  /** Contract owner can start the winner computation at any time. */
  @ContractTest(previous = "placeBidsOnContract")
  void startAuctionOnContract() {
    startAuction(owner);

    ZkSecondPriceAuctionExternalIds.ContractState state = auctionContract.getState().openState();

    Assertions.assertThat(state.auctionResult().secondHighestBid()).isEqualTo(256);
    Assertions.assertThat(state.auctionResult().winner().address()).isEqualTo(accounts.get(2));
    Assertions.assertThat(state.auctionResult().winner().externalId().idBytes())
        .containsExactly(0, 2);

    final var complexity = zkNodes.getComplexityOfLastComputation();
    Assertions.assertThat(complexity.numberOfRounds()).isEqualTo(364);
    Assertions.assertThat(complexity.multiplicationCount()).isEqualTo(1792);
  }

  /** Only the owner can register users. */
  @ContractTest(previous = "setupBidders")
  void nonOwnerFailsToRegisterUsers() {
    Assertions.assertThatCode(() -> registerBidders(accounts.get(6), List.of()))
        .hasMessageContaining("Only the owner can register bidders");
  }

  /** The same user cannot be registered twice. */
  @ContractTest(previous = "setupBidders")
  void registerTwice() {
    Assertions.assertThatCode(
            () ->
                registerBidders(
                    owner,
                    List.of(
                        new ZkSecondPriceAuctionExternalIds.AddressAndExternalId(
                            accounts.get(1), externalId(1)))))
        .hasMessageContaining("Duplicate bidder address");
  }

  /** Users can only bid once. */
  @ContractTest(previous = "placeBidsOnContract")
  void bidTwice() {
    Assertions.assertThatCode(() -> bidOnAuction(accounts.get(6), 2000))
        .hasMessageContaining("Each bidder is only allowed to place one bid");
  }

  /** Users must be registered to bid. */
  @ContractTest(previous = "deploy")
  void unregisteredBidder() {
    Assertions.assertThatCode(() -> bidOnAuction(accounts.get(6), 2000))
        .hasMessageContaining("is not a registered bidder");
  }

  /** Contract owner can start the winner computation when nobody has bid. */
  @ContractTest(previous = "deploy")
  void nonOwnerCannotStartAuction() {
    Assertions.assertThatCode(() -> startAuction(accounts.get(5)))
        .hasMessageContaining("Only contract owner can start the auction");
  }

  /** Contract owner can start the winner computation when nobody has bid. */
  @ContractTest(previous = "deploy")
  void startAuctionWithZeroBids() {
    Assertions.assertThatCode(() -> startAuction(owner))
        .hasMessageContaining(
            "At least 3 bidders must have submitted bids for the auction to start");
  }

  /** Contract owner can register which users can bid on the contract. */
  @ContractTest(previous = "startAuctionOnContract")
  void failToSetupBiddersAfterAuctionIsDone() {
    Assertions.assertThatCode(() -> registerBidders(owner, List.of()))
        .hasMessageContaining("Cannot register bidders after auction has begun");
  }

  /** Registered users can bid on the contract. */
  @ContractTest(previous = "startAuctionOnContract")
  void failTobidOnAuctionAfterAuctionIsDone() {
    Assertions.assertThatCode(() -> bidOnAuction(accounts.get(5), 256))
        .hasMessageContaining("Cannot place bid after auction has begun");
  }

  /** Contract owner can start the winner computation at any time. */
  @ContractTest(previous = "startAuctionOnContract")
  void failtostartAuctionAfterAuctionIsDone() {
    Assertions.assertThatCode(() -> startAuction(owner))
        .hasMessageContaining("Cannot start auction after it has already begun");
  }

  private void registerBidders(
      BlockchainAddress sender,
      List<ZkSecondPriceAuctionExternalIds.AddressAndExternalId> bidders) {
    blockchain.sendAction(
        sender, auctionContractAddress, ZkSecondPriceAuctionExternalIds.registerBidders(bidders));
  }

  private void bidOnAuction(BlockchainAddress bidder, int bidAmount) {
    CompactBitArray secretRpc =
        BitOutput.serializeBits(output -> output.writeUnsignedInt(bidAmount, 32));
    blockchain.sendSecretInput(auctionContractAddress, bidder, secretRpc, new byte[] {0x40});
  }

  private void startAuction(BlockchainAddress sender) {
    blockchain.sendAction(
        sender, auctionContractAddress, ZkSecondPriceAuctionExternalIds.startAuction());
  }
}
