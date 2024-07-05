package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkVotingSimple;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.language.junit.exceptions.SecretInputFailureException;
import com.partisiablockchain.language.testenvironment.zk.node.task.PendingInputId;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;

/** Test the Zero Knowledge Simple Voting Contract. */
public final class ZkVotingSimpleTest extends JunitContractTest {

  private static final ContractBytes VOTING_SIMPLE_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_voting_simple.zkwa"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_voting_simple.abi"),
          Path.of(
              "../rust/target/wasm32-unknown-unknown/release/zk_voting_simple_contract_runner"));

  private BlockchainAddress account1;
  private BlockchainAddress account2;
  private BlockchainAddress account3;
  private BlockchainAddress account4;
  private BlockchainAddress account5;
  private BlockchainAddress account6;

  private BlockchainAddress votingSimple;

  /** Deploy ZK voting contract. */
  @ContractTest
  void deploy() {
    account1 = blockchain.newAccount(2);
    account2 = blockchain.newAccount(3);
    account3 = blockchain.newAccount(4);
    account4 = blockchain.newAccount(5);
    account5 = blockchain.newAccount(6);
    account6 = blockchain.newAccount(7);

    byte[] initRpc = ZkVotingSimple.initialize(10000);

    votingSimple = blockchain.deployZkContract(account1, VOTING_SIMPLE_BYTES, initRpc);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(blockchain.getContractState(votingSimple));

    Assertions.assertThat(state).isNotNull();
  }

  /** The votes are counted correctly after the voting has ended. */
  @ContractTest(previous = "deploy")
  void countVotes() {
    // cast votes
    // "Yes"-votes
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), secretInputRpc());
    // "No"-votes
    blockchain.sendSecretInput(votingSimple, account2, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account3, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account4, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account5, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account6, createSecretIntInput(0), secretInputRpc());

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(10500);

    // count votes
    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();

    blockchain.sendAction(account1, votingSimple, startVoteCount);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(blockchain.getContractState(votingSimple));

    Assertions.assertThat(state.voteResult()).isEqualTo(new ZkVotingSimple.VoteResult(1, 5, false));
  }

  /** A proposal passes when there are strictly more "Yes"-votes than "No"-votes. */
  @ContractTest(previous = "deploy")
  void countVotesMajorityFor() {
    // "Yes"-votes
    blockchain.sendSecretInput(votingSimple, account2, createSecretIntInput(1), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account3, createSecretIntInput(1), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account4, createSecretIntInput(1), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account5, createSecretIntInput(1), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account6, createSecretIntInput(1), secretInputRpc());
    // No-votes
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(0), secretInputRpc());

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(10500);

    // count votes
    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();
    blockchain.sendAction(account1, votingSimple, startVoteCount);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(blockchain.getContractState(votingSimple));

    Assertions.assertThat(state.voteResult()).isEqualTo(new ZkVotingSimple.VoteResult(5, 1, true));
  }

  /** A proposal is rejected when the majority of the counted votes are "No"-votes. */
  @ContractTest(previous = "deploy")
  void countVotesMajorityAgainst() {
    // "Yes"-votes
    blockchain.sendSecretInput(votingSimple, account6, createSecretIntInput(1), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account4, createSecretIntInput(1), secretInputRpc());
    // "No"-votes
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account2, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account3, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account5, createSecretIntInput(0), secretInputRpc());

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(10500);

    // count votes
    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();
    blockchain.sendAction(account1, votingSimple, startVoteCount);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(blockchain.getContractState(votingSimple));

    Assertions.assertThat(state.voteResult()).isEqualTo(new ZkVotingSimple.VoteResult(2, 4, false));
  }

  /** The proposal fails if the voting ends in a draw. */
  @ContractTest(previous = "deploy")
  void countVotesDraw() {
    // "Yes"-votes
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account2, createSecretIntInput(1), secretInputRpc());
    // "No"-votes
    blockchain.sendSecretInput(votingSimple, account3, createSecretIntInput(0), secretInputRpc());
    blockchain.sendSecretInput(votingSimple, account4, createSecretIntInput(0), secretInputRpc());

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(10500);

    // count votes
    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();
    blockchain.sendAction(account1, votingSimple, startVoteCount);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(blockchain.getContractState(votingSimple));

    Assertions.assertThat(state.voteResult()).isEqualTo(new ZkVotingSimple.VoteResult(2, 2, false));
  }

  /** A user cannot cast a vote after the voting deadline has passed. */
  @ContractTest(previous = "deploy")
  void voterCannotVoteAfterDeadline() {
    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(10050);

    // cast vote
    Assertions.assertThatThrownBy(
            () ->
                blockchain.sendSecretInput(
                    votingSimple, account1, createSecretIntInput(1), secretInputRpc()))
        .isInstanceOf(SecretInputFailureException.class)
        .hasMessageContaining("Not allowed to vote after the deadline");
  }

  /** A voter can only cast one vote for each proposal. */
  @ContractTest(previous = "deploy")
  void eachVoterCanOnlyVoteOnce() {
    // cast vote
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), secretInputRpc());

    // cast another vote
    Assertions.assertThatThrownBy(
            () ->
                blockchain.sendSecretInput(
                    votingSimple, account1, createSecretIntInput(1), secretInputRpc()))
        .isInstanceOf(SecretInputFailureException.class)
        .hasMessageContaining("Each voter is only allowed to send one vote variable.");
  }

  /** A user cannot start the vote count before the voting deadline has passed. */
  @ContractTest(previous = "deploy")
  void startCountBeforeDeadline() {
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), secretInputRpc());

    blockchain.waitForBlockProductionTime(10000);

    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();

    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(account1, votingSimple, startVoteCount))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Vote counting cannot start before specified starting time");

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(blockchain.getContractState(votingSimple));

    Assertions.assertThat(state.voteResult()).isNull();
  }

  /** A user cannot start the vote count before the result of the vote has been calculated. */
  @ContractTest(previous = "deploy")
  void startCountFromCalculatingState() {
    zkNodes.stop();

    // cast vote
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), secretInputRpc());

    for (PendingInputId pendingInput : zkNodes.getPendingInputs(votingSimple)) {
      zkNodes.confirmInput(pendingInput);
    }

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(10050);
    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();

    // start counting votes
    blockchain.sendAction(account1, votingSimple, startVoteCount);

    // count votes in calculating state
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(account1, votingSimple, startVoteCount))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining("Vote counting must start from Waiting state, but was Calculating");

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(blockchain.getContractState(votingSimple));
    Assertions.assertThat(state.voteResult()).isNull();
  }

  /** A user cannot call for a recount of the votes for a proposal that has been decided. */
  @ContractTest(previous = "deploy")
  void startCountFromDoneState() {
    // cast vote
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), secretInputRpc());

    // pass time until past voting deadline
    blockchain.waitForBlockProductionTime(10050);
    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();

    blockchain.sendAction(account1, votingSimple, startVoteCount);

    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(account1, votingSimple, startVoteCount))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining("Vote counting must start from Waiting state, but was Done");
  }

  byte[] secretInputRpc() {
    return new byte[] {0x40};
  }

  CompactBitArray createSecretIntInput(int secret) {
    return BitOutput.serializeBits(output -> output.writeSignedInt(secret, 32));
  }
}
