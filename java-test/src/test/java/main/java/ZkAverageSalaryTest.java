package main.java;

import static org.assertj.core.api.Assertions.assertThat;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.AverageSalary;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.testenvironment.zk.node.task.PendingInputId;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import java.util.List;

/** Test suite for the ZkAverageSalary contract. */
public final class ZkAverageSalaryTest extends JunitContractTest {

  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/average_salary.zkwa"),
          Path.of("../target/wasm32-unknown-unknown/release/average_salary.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/average_salary_contract_runner"));

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

    byte[] contractState = blockchain.getContractState(averageSalary);

    AverageSalary.ContractState state = AverageSalary.ContractState.deserialize(contractState);

    assertThat(state.administrator()).isEqualTo(account1);
  }

  /** Sends secret input to the contract. */
  @ContractTest(previous = "deployZkContract")
  public void sendSecretInput() {

    byte[] contractState = blockchain.getContractState(averageSalary);

    AverageSalary.ContractState state = AverageSalary.ContractState.deserialize(contractState);
    assertThat(state).isNotNull();

    blockchain.sendSecretInput(
        averageSalary, account1, createSecretIntInput(5000), new byte[] {0x40});
    blockchain.sendSecretInput(
        averageSalary, account2, createSecretIntInput(20000), new byte[] {0x40});
    blockchain.sendSecretInput(
        averageSalary, account3, createSecretIntInput(15000), new byte[] {0x40});
  }

  /** Sends secret input to the contract several times. */
  @ContractTest(previous = "deployZkContract")
  public void sendManyInputs() {
    zkNodes.stop();

    PendingInputId account1Input =
        blockchain.sendSecretInput(
            averageSalary, account1, createSecretIntInput(100000), new byte[] {0x40});
    zkNodes.confirmInput(account1Input);

    PendingInputId account2Input =
        blockchain.sendSecretInput(
            averageSalary, account2, createSecretIntInput(10000), new byte[] {0x40});
    zkNodes.confirmInput(account2Input);

    PendingInputId account3Input =
        blockchain.sendSecretInput(
            averageSalary, account3, createSecretIntInput(3000), new byte[] {0x40});
    zkNodes.confirmInput(account3Input);

    PendingInputId account4Input =
        blockchain.sendSecretInput(
            averageSalary, account4, createSecretIntInput(15000), new byte[] {0x40});
    zkNodes.confirmInput(account4Input);

    PendingInputId account5Input =
        blockchain.sendSecretInput(
            averageSalary, account5, createSecretIntInput(23300), new byte[] {0x40});
    zkNodes.confirmInput(account5Input);

    blockchain.sendSecretInput(
        averageSalary, account6, createSecretIntInput(40150), new byte[] {0x40});
  }

  /** Starts the ZK computation. */
  @ContractTest(previous = "sendSecretInput")
  public void startComputation() {
    byte[] startCompute = AverageSalary.computeAverageSalary();
    blockchain.sendAction(account1, averageSalary, startCompute);
    AverageSalary.ContractState state =
        AverageSalary.ContractState.deserialize(blockchain.getContractState(averageSalary));
    assertThat(state.averageSalaryResult()).isEqualTo(13333);
  }

  /** Starts and completes the ZK computation. */
  @ContractTest(previous = "sendManyInputs")
  public void startComputationWithAllInput() {
    zkNodes.stop();
    List<PendingInputId> pendingInputs = zkNodes.getPendingInputs(averageSalary);

    assertThat(pendingInputs.size()).isEqualTo(1);

    zkNodes.confirmInput(pendingInputs.get(0));

    byte[] startCompute = AverageSalary.computeAverageSalary();
    blockchain.sendAction(account1, averageSalary, startCompute);
    zkNodes.finishTasks();
    AverageSalary.ContractState state =
        AverageSalary.ContractState.deserialize(blockchain.getContractState(averageSalary));

    assertThat(state.averageSalaryResult()).isEqualTo(31908);
  }

  @ContractTest(previous = "sendManyInputs")
  void startComputationMissingOneInput() {
    zkNodes.stop();
    byte[] startCompute = AverageSalary.computeAverageSalary();
    blockchain.sendAction(account1, averageSalary, startCompute);

    zkNodes.zkCompute(averageSalary);
    zkNodes.finishTasks();

    AverageSalary.ContractState state =
        AverageSalary.ContractState.deserialize(blockchain.getContractState(averageSalary));
    assertThat(state.averageSalaryResult()).isEqualTo(30260);
  }

  CompactBitArray createSecretIntInput(int secret) {
    return BitOutput.serializeBits(output -> output.writeSignedInt(secret, 32));
  }
}
