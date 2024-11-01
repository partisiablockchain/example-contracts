package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.Dns;
import com.partisiablockchain.language.abicodegen.DnsVotingClient;
import com.partisiablockchain.language.abicodegen.Voting;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import org.assertj.core.api.Assertions;

/** Test suite for the DNS voting client contract. */
public final class DnsVotingClientTest extends JunitContractTest {
  private static final ContractBytes DNS_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns_runner"));

  private static final ContractBytes DNS_VOTING_CLIENT_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns_voting_client.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns_voting_client.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns_voting_client_runner"));

  private static final ContractBytes VOTING_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting_runner"));

  private BlockchainAddress voter;
  private BlockchainAddress admin;
  private BlockchainAddress dnsVotingClientAddress;
  private BlockchainAddress dnsAddress;
  private Voting votingContract;

  /**
   * Setup for all the other tests. Instantiates two accounts, one voter and one admin, and deploys
   * a DNS contract, a DNS client contract, and a voting contract.
   */
  @ContractTest
  void setUp() {
    voter = blockchain.newAccount(2);
    admin = blockchain.newAccount(3);

    byte[] initDnsRpc = Dns.initialize();
    dnsAddress = blockchain.deployContract(voter, DNS_CONTRACT_BYTES, initDnsRpc);

    byte[] initDnsClientRpc = DnsVotingClient.initialize(dnsAddress);
    dnsVotingClientAddress =
        blockchain.deployContract(voter, DNS_VOTING_CLIENT_CONTRACT_BYTES, initDnsClientRpc);

    byte[] initVotingRpc = Voting.initialize(10, List.of(dnsVotingClientAddress), 60 * 60 * 1000);
    BlockchainAddress votingAddress =
        blockchain.deployContract(admin, VOTING_CONTRACT_BYTES, initVotingRpc);
    votingContract = new Voting(getStateClient(), votingAddress);

    byte[] registerRpc = Dns.registerDomain("voting", votingAddress);
    blockchain.sendAction(admin, dnsAddress, registerRpc);
  }

  /**
   * A user can cast a vote on the voting client by specifying the domain of the voting contract.
   */
  @ContractTest(previous = "setUp")
  public void vote() {
    byte[] voteRpc = DnsVotingClient.vote("voting", true);
    blockchain.sendAction(voter, dnsVotingClientAddress, voteRpc);

    Map<BlockchainAddress, Boolean> castVotes = votingContract.getState().votes();

    Assertions.assertThat(castVotes).isEqualTo(Map.of(dnsVotingClientAddress, true));
  }

  /** When the user votes on a different voting domain, the vote will go to that domain. */
  @ContractTest(previous = "setUp")
  public void voteDifferentDomains() {
    byte[] initVoterRpc2 = Voting.initialize(11, List.of(dnsVotingClientAddress), 60 * 60 * 1000);
    BlockchainAddress voting2 =
        blockchain.deployContract(admin, VOTING_CONTRACT_BYTES, initVoterRpc2);
    Voting votingContract2 = new Voting(getStateClient(), voting2);

    byte[] registerRpc2 = Dns.registerDomain("voting2", voting2);
    blockchain.sendAction(admin, dnsAddress, registerRpc2);

    byte[] voteRpc = DnsVotingClient.vote("voting2", true);
    blockchain.sendAction(voter, dnsVotingClientAddress, voteRpc);

    Map<BlockchainAddress, Boolean> castVotes2 = votingContract2.getState().votes();
    Assertions.assertThat(castVotes2).isEqualTo(Map.of(dnsVotingClientAddress, true));

    Map<BlockchainAddress, Boolean> castVotes = votingContract.getState().votes();
    Assertions.assertThat(castVotes).isEmpty();
  }

  /** A user cannot cast a vote if the voting domain is not registered in the DNS. */
  @ContractTest(previous = "setUp")
  public void voteBadDomain() {
    byte[] voteRpc = DnsVotingClient.vote("baddomain", true);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(voter, dnsVotingClientAddress, voteRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("No address found with the given domain");
  }
}
