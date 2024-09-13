package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.MultiVotingContract;
import com.partisiablockchain.language.abicodegen.Voting;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.language.testenvironment.TxExecution;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;

/** This class contains a test suite for the multi-voting smart contract. */
public final class MultiVotingTest extends JunitContractTest {
  private static final ContractBytes MULTI_VOTING_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/multi_voting_contract.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/multi_voting_contract.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/multi_voting_contract_runner"));

  private static final ContractBytes VOTING_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/voting.abi"));

  private BlockchainAddress multiVotingOwner;
  private BlockchainAddress multiVoting;
  private BlockchainAddress voter1;
  private BlockchainAddress voter2;

  /**
   * Root test that provides setup for all other tests. Instantiates the owner of the multi voting
   * contract, deploys the multi voting contract and instantiates two voters.
   */
  @ContractTest
  public void setup() {
    multiVotingOwner = blockchain.newAccount(1);
    byte[] multiVotingInitRpc =
        MultiVotingContract.initialize(
            VOTING_CONTRACT_BYTES.code(), VOTING_CONTRACT_BYTES.abi(), 1);
    multiVoting =
        blockchain.deployContract(
            multiVotingOwner, MULTI_VOTING_CONTRACT_BYTES, multiVotingInitRpc);

    voter1 = blockchain.newAccount(2);
    voter2 = blockchain.newAccount(3);
  }

  // Feature: Deploy Voting Contract

  /** The multi-voting contract can deploy a new voting contract for a proposal. */
  @ContractTest(previous = "setup")
  public void deployVotingContract() {
    byte[] deployVotingContractRpc = MultiVotingContract.addVotingContract(10, 60 * 60 * 1000);
    blockchain.sendAction(multiVotingOwner, multiVoting, deployVotingContractRpc);
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    Assertions.assertThat(state.votingContracts().size()).isEqualTo(1);
    Assertions.assertThat(state.votingContracts().get((long) 10)).isNotNull();
  }

  /** Only the owner is allowed to deploy new voting contracts. */
  @ContractTest(previous = "setup")
  public void nonOwnerDeployVotingContract() {
    byte[] deployVotingContractRpc = MultiVotingContract.addVotingContract(10, 60 * 60 * 1000);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(voter1, multiVoting, deployVotingContractRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only owner can add contracts");
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    Assertions.assertThat(state.votingContracts().size()).isEqualTo(0);
  }

  /** Only a single voting contract can be deployed for each proposal ID. */
  @ContractTest(previous = "setup")
  public void deployVotingContractTwice() {
    byte[] deployVotingContractRpc = MultiVotingContract.addVotingContract(10, 60 * 60 * 1000);
    TxExecution execution =
        blockchain.sendAction(multiVotingOwner, multiVoting, deployVotingContractRpc);

    execution.printGasAccounting();
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(multiVotingOwner, multiVoting, deployVotingContractRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Proposal id already exists");
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    Assertions.assertThat(state.votingContracts().size()).isEqualTo(1);
    Assertions.assertThat(state.votingContracts().get((long) 10)).isNotNull();
  }

  /** If there are no eligible voters it is not possible to deploy a new voting contract. */
  @ContractTest(previous = "setup")
  public void deployVotingContractNoVoters() {
    byte[] removeVoterRpc = MultiVotingContract.removeVoter(multiVotingOwner);
    blockchain.sendAction(multiVotingOwner, multiVoting, removeVoterRpc);
    byte[] deployVotingContractRpc = MultiVotingContract.addVotingContract(10, 60 * 60 * 1000);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(multiVotingOwner, multiVoting, deployVotingContractRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Voters are required");
  }

  /** The multi-voting contract cannot deploy a voting contract with insufficient gas. */
  @ContractTest(previous = "setup")
  public void deployVotingContractNotEnoughGas() {
    byte[] addVotingContractRpc = MultiVotingContract.addVotingContract(12, 1000);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(multiVotingOwner, multiVoting, addVotingContractRpc, 1200))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Out of instruction cycles!");
  }

  /** The multi-voting contract cannot deploy a voting contract with incorrectly formatted WASM. */
  @ContractTest(previous = "setup")
  public void deployVotingContractIncorrectWasm() {
    byte[] multiVotingInitRpc =
        MultiVotingContract.initialize(new byte[] {}, VOTING_CONTRACT_BYTES.abi(), 1);
    BlockchainAddress multiVotingContract =
        blockchain.deployContract(
            multiVotingOwner, MULTI_VOTING_CONTRACT_BYTES, multiVotingInitRpc);
    byte[] addVotingContractRpc = MultiVotingContract.addVotingContract(11, 1000);
    Assertions.assertThatThrownBy(
            () ->
                blockchain.sendAction(multiVotingOwner, multiVotingContract, addVotingContractRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Unable to instantiate handler");
  }

  /** The multi-voting contract cannot deploy a voting contract with a non-existent binder id. */
  @ContractTest(previous = "setup")
  public void deployVotingContractIncorrectBinderId() {
    byte[] multiVotingInitRpc =
        MultiVotingContract.initialize(
            VOTING_CONTRACT_BYTES.code(), VOTING_CONTRACT_BYTES.abi(), 42);
    BlockchainAddress multiVotingContract =
        blockchain.deployContract(
            multiVotingOwner, MULTI_VOTING_CONTRACT_BYTES, multiVotingInitRpc);
    byte[] addVotingContractRpc = MultiVotingContract.addVotingContract(11, 1000);
    Assertions.assertThatThrownBy(
            () ->
                blockchain.sendAction(multiVotingOwner, multiVotingContract, addVotingContractRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("PublicDeployContractState.getBinderInfo(int)\" is null");
  }

  // Feature: Add Voter

  /** The multi-voting contract can add users as registered voters. */
  @ContractTest(previous = "setup")
  public void addVoterToVotingContract() {
    byte[] addVoterRpc = MultiVotingContract.addVoter(voter1);
    blockchain.sendAction(multiVotingOwner, multiVoting, addVoterRpc);
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    Assertions.assertThat(state.eligibleVoters().size())
        .isEqualTo(2); // newly added voter and contract owner
    Assertions.assertThat(state.eligibleVoters().contains(voter1)).isTrue();
  }

  /** Users can only be added as registered voters once. */
  @ContractTest(previous = "setup")
  public void addVoterTwice() {
    byte[] addVoterRpc = MultiVotingContract.addVoter(voter1);
    blockchain.sendAction(multiVotingOwner, multiVoting, addVoterRpc);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(multiVotingOwner, multiVoting, addVoterRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Voter already exists");
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    // check that correctly added voter still there
    Assertions.assertThat(state.eligibleVoters().size()).isEqualTo(2);
    Assertions.assertThat(state.eligibleVoters().contains(voter1)).isTrue();
  }

  /** Users can only be added as registered voters by the owner of the multi-voting contract. */
  @ContractTest(previous = "setup")
  public void nonOwnerAddVoter() {
    byte[] addVoterRpc = MultiVotingContract.addVoter(voter1);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter1, multiVoting, addVoterRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only owner can add voters");
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    Assertions.assertThat(state.eligibleVoters().size()).isEqualTo(1);
    Assertions.assertThat(state.eligibleVoters().contains(voter1)).isFalse();
  }

  /** Eligible voters are correctly registered in the deployed voting contracts. */
  @ContractTest(previous = "setup")
  public void addedVotersAreRegisteredInDeployedContract() {
    byte[] addVoterOneRpc = MultiVotingContract.addVoter(voter1);
    blockchain.sendAction(multiVotingOwner, multiVoting, addVoterOneRpc);
    byte[] addVoterTwoRpc = MultiVotingContract.addVoter(voter2);
    blockchain.sendAction(multiVotingOwner, multiVoting, addVoterTwoRpc);
    byte[] deployVotingContractRpc = MultiVotingContract.addVotingContract(12, 60 * 60 * 1000);
    blockchain.sendAction(multiVotingOwner, multiVoting, deployVotingContractRpc);
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    BlockchainAddress votingContractAddress = state.votingContracts().get((long) 12);
    Voting.VoteState voteState =
        Voting.VoteState.deserialize(blockchain.getContractState(votingContractAddress));
    Assertions.assertThat(voteState.voters().contains(voter1)).isTrue();
    Assertions.assertThat(voteState.voters().contains(voter2)).isTrue();
  }

  /** Newly added voters are not registered in previously deployed voting contracts. */
  @ContractTest(previous = "setup")
  public void newVotersAreNotRegisteredInPreviouslyDeployedContract() {
    byte[] addVoterOneRpc = MultiVotingContract.addVoter(voter1);
    blockchain.sendAction(multiVotingOwner, multiVoting, addVoterOneRpc);
    byte[] deployVotingContractRpc = MultiVotingContract.addVotingContract(12, 60 * 60 * 1000);
    blockchain.sendAction(multiVotingOwner, multiVoting, deployVotingContractRpc);
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    BlockchainAddress votingContractAddress = state.votingContracts().get((long) 12);
    Voting.VoteState voteState =
        Voting.VoteState.deserialize(blockchain.getContractState(votingContractAddress));
    Assertions.assertThat(voteState.voters().contains(voter1)).isTrue();

    byte[] addVoterTwoRpc = MultiVotingContract.addVoter(voter2);
    blockchain.sendAction(multiVotingOwner, multiVoting, addVoterTwoRpc);

    voteState = Voting.VoteState.deserialize(blockchain.getContractState(votingContractAddress));
    Assertions.assertThat(voteState.voters().contains(voter2)).isFalse();
  }

  // Feature: Remove Voter

  /** The multi-voting contract can remove users as registered voters. */
  @ContractTest(previous = "addVoterToVotingContract")
  public void removeVoter() {
    byte[] removeVoterRpc = MultiVotingContract.removeVoter(voter1);
    blockchain.sendAction(multiVotingOwner, multiVoting, removeVoterRpc);
    MultiVotingContract.MultiVotingState state =
        MultiVotingContract.MultiVotingState.deserialize(blockchain.getContractState(multiVoting));
    Assertions.assertThat(state.eligibleVoters().size())
        .isEqualTo(1); // newly added voter and contract owner
    Assertions.assertThat(state.eligibleVoters().contains(voter1)).isFalse();
  }

  /** Users can only be removed as registered voters by the owner of the multi-voting contract. */
  @ContractTest(previous = "addVoterToVotingContract")
  public void nonOwnerRemoveVoter() {
    byte[] removeVoterRpc = MultiVotingContract.removeVoter(voter1);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter2, multiVoting, removeVoterRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only owner can remove voters");
  }
}
