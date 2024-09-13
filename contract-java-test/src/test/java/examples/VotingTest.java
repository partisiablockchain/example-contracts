package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.Voting;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import java.nio.file.Path;
import java.util.List;
import org.assertj.core.api.Assertions;

/** Test suite for the Voting contract. */
public final class VotingTest extends JunitContractTest {

  private static final ContractBytes VOTING_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting_runner"));
  private BlockchainAddress voter1;
  private BlockchainAddress voter2;
  private BlockchainAddress voter3;
  private BlockchainAddress voting;

  /** Setup for all the other tests. Deploys a voting contract and instantiates accounts. */
  @ContractTest
  void setUp() {
    voter1 = blockchain.newAccount(2);
    voter2 = blockchain.newAccount(3);
    voter3 = blockchain.newAccount(4);

    byte[] initRpc = Voting.initialize(10, List.of(voter1, voter2, voter3), 60 * 60 * 1000);
    voting = blockchain.deployContract(voter1, VOTING_CONTRACT_BYTES, initRpc);
  }

  /** An eligible voter can cast a vote. */
  @ContractTest(previous = "setUp")
  public void castVote() {
    byte[] votingRpc = Voting.vote(true);
    blockchain.sendAction(voter1, voting, votingRpc);
    Voting.VoteState state = Voting.VoteState.deserialize(blockchain.getContractState(voting));

    Assertions.assertThat(state.votes().size()).isEqualTo(1);
    Assertions.assertThat(state.votes().get(voter1)).isTrue();
  }

  /**
   * When the majority votes yes (strictly more yes votes than no votes) the result of the count is
   * that the proposal passes.
   */
  @ContractTest(previous = "setUp")
  public void countVotesMajorityFor() {
    // cast votes
    byte[] voteRpc = Voting.vote(true);
    blockchain.sendAction(voter1, voting, voteRpc);
    blockchain.sendAction(voter2, voting, voteRpc);
    blockchain.sendAction(voter3, voting, voteRpc);

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(2 * 60 * 60 * 1000);

    // count votes
    byte[] countRpc = Voting.count();
    blockchain.sendAction(voter1, voting, countRpc);

    Voting.VoteState state = Voting.VoteState.deserialize(blockchain.getContractState(voting));
    Assertions.assertThat(state.result()).isTrue();
  }

  /** If the majority votes no, the result of the count is that the proposal fails. */
  @ContractTest(previous = "setUp")
  public void countVotesMajorityAgainst() {
    // cast votes
    byte[] voteAgainstRpc = Voting.vote(false);
    blockchain.sendAction(voter1, voting, voteAgainstRpc);
    blockchain.sendAction(voter2, voting, voteAgainstRpc);
    byte[] voteForRpc = Voting.vote(true);
    blockchain.sendAction(voter3, voting, voteForRpc);

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(2 * 60 * 60 * 1000);

    // count votes
    byte[] countRpc = Voting.count();
    blockchain.sendAction(voter1, voting, countRpc);

    Voting.VoteState state = Voting.VoteState.deserialize(blockchain.getContractState(voting));
    Assertions.assertThat(state.result()).isFalse();
  }

  /** If the voting ends in a draw, the result of the count is that the proposal fails. */
  @ContractTest(previous = "setUp")
  public void draw() {

    BlockchainAddress voter4 = blockchain.newAccount(5);

    byte[] initRpc = Voting.initialize(11, List.of(voter1, voter2, voter3, voter4), 60 * 60 * 1000);
    voting = blockchain.deployContract(voter1, VOTING_CONTRACT_BYTES, initRpc);
    // cast votes
    byte[] voteAgainstRpc = Voting.vote(false);
    blockchain.sendAction(voter1, voting, voteAgainstRpc);
    blockchain.sendAction(voter2, voting, voteAgainstRpc);
    byte[] voteForRpc = Voting.vote(true);
    blockchain.sendAction(voter3, voting, voteForRpc);
    blockchain.sendAction(voter4, voting, voteForRpc);

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(2 * 60 * 60 * 1000);

    // count votes
    byte[] countRpc = Voting.count();
    blockchain.sendAction(voter1, voting, countRpc);

    Voting.VoteState state = Voting.VoteState.deserialize(blockchain.getContractState(voting));
    Assertions.assertThat(state.result()).isFalse();
  }

  /** Two identical votes sent from the same voter is only counted as one. */
  @ContractTest(previous = "setUp")
  public void voteTwice() {
    byte[] votingRpc = Voting.vote(true);
    blockchain.sendAction(voter1, voting, votingRpc);
    Voting.VoteState state = Voting.VoteState.deserialize(blockchain.getContractState(voting));
    Assertions.assertThat(state.votes().size()).isEqualTo(1);

    blockchain.sendAction(voter1, voting, votingRpc);
    state = Voting.VoteState.deserialize(blockchain.getContractState(voting));
    Assertions.assertThat(state.votes().size()).isEqualTo(1);
  }

  /** Two different votes from the same voter is only registered as the last vote. */
  @ContractTest(previous = "setUp")
  public void changeVote() {
    byte[] voteTrueRpc = Voting.vote(true);
    blockchain.sendAction(voter1, voting, voteTrueRpc);
    Voting.VoteState state = Voting.VoteState.deserialize(blockchain.getContractState(voting));
    Assertions.assertThat(state.votes().size()).isEqualTo(1);

    byte[] voteFalseRpc = Voting.vote(false);
    blockchain.sendAction(voter1, voting, voteFalseRpc);
    state = Voting.VoteState.deserialize(blockchain.getContractState(voting));
    Assertions.assertThat(state.votes().size()).isEqualTo(1);
  }

  /** Deployment of a voting contract without any eligible voters fails. */
  @ContractTest(previous = "setUp")
  public void deployWithoutVoters() {
    byte[] initRpc = Voting.initialize(11, List.of(), 60 * 60 * 1000);
    Assertions.assertThatThrownBy(
            () -> blockchain.deployContract(voter1, VOTING_CONTRACT_BYTES, initRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Voters are required");
  }

  /** Deployment of a voting contract with duplicate voter in the eligible voter list fails. */
  @ContractTest(previous = "setUp")
  public void deployWithNonUniqueVoters() {
    byte[] initRpc = Voting.initialize(11, List.of(voter2, voter2), 60 * 60 * 1000);
    Assertions.assertThatThrownBy(
            () -> blockchain.deployContract(voter1, VOTING_CONTRACT_BYTES, initRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("All voters must be unique");
  }

  /** Casting a vote after the deadline fails. */
  @ContractTest(previous = "setUp")
  public void voteAfterDeadline() {
    byte[] voteRpc = Voting.vote(true);
    blockchain.waitForBlockProductionTime(2 * 60 * 60 * 1000);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter1, voting, voteRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("The deadline has passed");
  }

  /** Casting a vote from an unregistered voter fails. */
  @ContractTest(previous = "setUp")
  public void voteCastFromNonRegisteredAccount() {
    byte[] voteRpc = Voting.vote(true);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(blockchain.newAccount(15), voting, voteRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Not an eligible voter");
  }

  /** Calling for a recount fails. */
  @ContractTest(previous = "countVotesMajorityFor")
  public void countVotesTwice() {
    byte[] countRpc = Voting.count();
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter1, voting, countRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("The votes have already been counted");
  }

  /** Counting the votes before the deadline has passed fails. */
  @ContractTest(previous = "setUp")
  public void countBeforeDeadline() {
    // cast votes
    byte[] voteRpc = Voting.vote(true);
    blockchain.sendAction(voter1, voting, voteRpc);
    blockchain.sendAction(voter2, voting, voteRpc);
    blockchain.sendAction(voter3, voting, voteRpc);

    // count votes
    byte[] countRpc = Voting.count();
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter1, voting, countRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("The deadline has not yet passed");
  }
}
