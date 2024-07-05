package examples;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.AverageSalary;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.language.junit.exceptions.SecretInputFailureException;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;

/** Test suite for the ZkAverageSalary contract. */
public final class ZkAverageSalaryTest extends JunitContractTest {

  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/average_salary.zkwa"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/average_salary.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/average_salary_contract_runner"));

  private BlockchainAddress account1;
  private BlockchainAddress account2;
  private BlockchainAddress account3;
  private BlockchainAddress account4;
  private BlockchainAddress account5;
  private BlockchainAddress account6;
  private BlockchainAddress averageSalary;

  /** Deploys the contract. */
  @ContractTest
  public void deployZkContract() {
    account1 = blockchain.newAccount(2);
    account2 = blockchain.newAccount(3);
    account3 = blockchain.newAccount(4);
    account4 = blockchain.newAccount(5);
    account5 = blockchain.newAccount(6);
    account6 = blockchain.newAccount(7);

    byte[] initialize = AverageSalary.initialize();

    averageSalary = blockchain.deployZkContract(account1, CONTRACT_BYTES, initialize);

    AverageSalary.ContractState state =
        AverageSalary.ContractState.deserialize(blockchain.getContractState(averageSalary));

    assertThat(state.administrator()).isEqualTo(account1);
  }

  /** A user can input their salary. */
  @ContractTest(previous = "deployZkContract")
  public void sendSecretInputs() {
    blockchain.sendSecretInput(
        averageSalary, account1, createSecretIntInput(100000), secretInputRpc());
    blockchain.sendSecretInput(
        averageSalary, account2, createSecretIntInput(10000), secretInputRpc());
    blockchain.sendSecretInput(
        averageSalary, account3, createSecretIntInput(3000), secretInputRpc());
    blockchain.sendSecretInput(
        averageSalary, account4, createSecretIntInput(15000), secretInputRpc());
    blockchain.sendSecretInput(
        averageSalary, account5, createSecretIntInput(23300), secretInputRpc());
    blockchain.sendSecretInput(
        averageSalary, account6, createSecretIntInput(40150), secretInputRpc());
  }

  /** A user can input 0 as their salary. */
  @ContractTest(previous = "deployZkContract")
  public void sendInputsIncludingZero() {
    blockchain.sendSecretInput(
        averageSalary, account1, createSecretIntInput(100000), secretInputRpc());
    blockchain.sendSecretInput(
        averageSalary, account2, createSecretIntInput(10000), secretInputRpc());
    blockchain.sendSecretInput(averageSalary, account3, createSecretIntInput(0), secretInputRpc());
  }

  /** The computation of the average should yield the correct average. */
  @ContractTest(previous = "sendSecretInputs")
  void calculateAverageSalary() {
    byte[] startCompute = AverageSalary.computeAverageSalary();
    blockchain.sendAction(account1, averageSalary, startCompute);

    AverageSalary.ContractState state =
        AverageSalary.ContractState.deserialize(blockchain.getContractState(averageSalary));

    assertThat(state.averageSalaryResult())
        .isEqualTo((100000 + 10000 + 3000 + 15000 + 23300 + 40150) / 6);
  }

  /** The calculation of the average allows salary inputs, where the inputted salary is 0. */
  @ContractTest(previous = "sendInputsIncludingZero")
  public void averageSalaryIncludingZero() {
    byte[] startCompute = AverageSalary.computeAverageSalary();
    blockchain.sendAction(account1, averageSalary, startCompute);

    AverageSalary.ContractState state =
        AverageSalary.ContractState.deserialize(blockchain.getContractState(averageSalary));

    assertThat(state.averageSalaryResult()).isEqualTo((100000 + 10000 + 0) / 3);
  }

  /** Each employee is only allowed to submit their salary once. */
  @ContractTest(previous = "deployZkContract")
  void eachEmployeeCanOnlySendInputOnce() {
    blockchain.sendSecretInput(
        averageSalary, account1, createSecretIntInput(100000), secretInputRpc());
    assertThatThrownBy(
            () ->
                blockchain.sendSecretInput(
                    averageSalary, account1, createSecretIntInput(100000), secretInputRpc()))
        .isInstanceOf(SecretInputFailureException.class)
        .hasMessageContaining("Each address is only allowed to send one salary variable.");
  }

  /** Only the admin is allowed to start the computation. */
  @ContractTest(previous = "sendSecretInputs")
  void onlyAdministratorCanStartComputation() {
    AverageSalary.ContractState state =
        AverageSalary.ContractState.deserialize(blockchain.getContractState(averageSalary));

    assertThat(state.administrator()).isNotEqualTo(account2);

    byte[] startCompute = AverageSalary.computeAverageSalary();

    assertThatThrownBy(() -> blockchain.sendAction(account2, averageSalary, startCompute))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only administrator can start computation");
  }

  /** The admin cannot start another computation, if there is already a computation running. */
  @ContractTest(previous = "sendSecretInputs")
  void computationCanNotStartInCalculatingState() {
    zkNodes.stop();

    byte[] startCompute = AverageSalary.computeAverageSalary();
    blockchain.sendAction(account1, averageSalary, startCompute);

    assertThatThrownBy(() -> blockchain.sendAction(account1, averageSalary, startCompute))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Computation must start from Waiting state, but was Calculating");
  }

  /** The average salary can only be computed once. */
  @ContractTest(previous = "sendSecretInputs")
  void computationCanNotStartInDoneState() {
    byte[] startCompute = AverageSalary.computeAverageSalary();
    blockchain.sendAction(account1, averageSalary, startCompute);

    assertThatThrownBy(() -> blockchain.sendAction(account1, averageSalary, startCompute))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Computation must start from Waiting state, but was Done");
  }

  /**
   * The admin can only start the computation if there is at least 3 inputs to the contract, to keep
   * the inputs hidden in the result.
   */
  @ContractTest(previous = "deployZkContract")
  void computationNotEnoughInput() {
    blockchain.sendSecretInput(
        averageSalary, account1, createSecretIntInput(100000), secretInputRpc());
    blockchain.sendSecretInput(
        averageSalary, account2, createSecretIntInput(10000), secretInputRpc());

    byte[] startCompute = AverageSalary.computeAverageSalary();

    assertThatThrownBy(() -> blockchain.sendAction(account1, averageSalary, startCompute))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "At least 3 employees must have submitted and confirmed their inputs, before starting"
                + " computation, but had only 2");
  }

  CompactBitArray createSecretIntInput(int secret) {
    return BitOutput.serializeBits(output -> output.writeSignedInt(secret, 32));
  }

  byte[] secretInputRpc() {
    return new byte[] {0x40};
  }
}
