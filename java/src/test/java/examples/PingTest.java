package examples;

import static org.assertj.core.api.Assertions.within;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.PingContract;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.FuzzyState;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.math.Unsigned256;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;

/** {@link PingContract} testing. */
public final class PingTest extends JunitContractTest {

  /** {@link PingContract} contract bytes. */
  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/ping_contract.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/ping_contract.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/ping_contract_runner"));

  private static final long CONTRACT_STORAGE_FEE = 1;

  private BlockchainAddress user;

  private BlockchainAddress pingContractAddress;

  private BlockchainAddress differentContractAddress;

  private static final long DEFAULT_CALL_COST = 10_000_000L;
  private static final long NETWORK_FEE = 130L;

  // An address where no contract lives.
  private final BlockchainAddress wrongAddress =
      BlockchainAddress.fromString(
          "02DEADBEEF12345678DEADBEEF87654321DEADBEEFDEADBEEFDEADBEEFDEADBEEF");

  /** Contract can be correctly deployed. */
  @ContractTest
  void deploy() {
    user = blockchain.newAccount(3);

    // Deploy contracts.
    byte[] initRpc = PingContract.initialize();
    pingContractAddress = blockchain.deployContract(user, CONTRACT_BYTES, initRpc);

    // For testing, ensure contracts are not the same.
    differentContractAddress = blockchain.deployContract(user, CONTRACT_BYTES, initRpc);
    Assertions.assertThat(differentContractAddress).isNotEqualTo(pingContractAddress);

    // Get the main contract's state.
    PingContract.PingContractState state =
        PingContract.PingContractState.deserialize(
            blockchain.getContractState(pingContractAddress));

    // Contract state is correctly initialized.
    Assertions.assertThat(state).isNotNull();
  }

  /** Pinging a different existing contract will not result in any error. */
  @ContractTest(previous = "deploy")
  void simplePing() {
    ping(differentContractAddress, null);
  }

  /** Pinging the contract itself with not result in error. */
  @ContractTest(previous = "deploy")
  void simplePingSelf() {
    ping(pingContractAddress, null);
  }

  /** Ping interaction can be used to transfer gas to a different existing contract. */
  @ContractTest(previous = "deploy")
  void sendGas() {
    final long gasToSend = 1000L;
    // Gas for running instructions on the ping contract.
    final long callOverhead = 1100L;

    final long userPreAmount = getUserGasBalance(user);
    final long internalPreAmount = getContractGasBalance(pingContractAddress);
    final long externalPreAmount = getContractGasBalance(differentContractAddress);

    ping(differentContractAddress, gasToSend, gasToSend + callOverhead);

    // Get new state
    final long userPostAmount = getUserGasBalance(user);
    final long internalPostAmount = getContractGasBalance(pingContractAddress);
    final long externalPostAmount = getContractGasBalance(differentContractAddress);

    // Check that gas is as expected.
    Assertions.assertThat(userPostAmount).isEqualTo(userPreAmount - gasToSend - callOverhead);
    Assertions.assertThat(internalPostAmount).isCloseTo(internalPreAmount, within(NETWORK_FEE));
    Assertions.assertThat(externalPostAmount)
        .isEqualTo(externalPreAmount + gasToSend - NETWORK_FEE - CONTRACT_STORAGE_FEE);
  }

  /**
   * Ping interaction without callback can be used to transfer gas to a different existing contract,
   * without checking if it exists.
   */
  @ContractTest(previous = "deploy")
  void sendGasWithoutCallback() {
    final long gasToSend = 1000L;
    // Gas for running instructions on the ping contract.
    final long callOverhead = 1100L;

    // Save state before ping
    final long userPreAmount = getUserGasBalance(user);
    final long internalPreAmount = getContractGasBalance(pingContractAddress);
    final long externalPreAmount = getContractGasBalance(differentContractAddress);

    // Call ping functionality
    pingNoCallback(differentContractAddress, gasToSend, gasToSend + callOverhead);

    // Get new state
    final long userPostAmount = getUserGasBalance(user);
    final long internalPostAmount = getContractGasBalance(pingContractAddress);
    final long externalPostAmount = getContractGasBalance(differentContractAddress);

    // Check that gas is as expected.
    Assertions.assertThat(userPostAmount).isEqualTo(userPreAmount - gasToSend - callOverhead);
    Assertions.assertThat(internalPostAmount).isCloseTo(internalPreAmount, within(NETWORK_FEE));
    Assertions.assertThat(externalPostAmount)
        .isEqualTo(externalPreAmount + gasToSend - NETWORK_FEE - CONTRACT_STORAGE_FEE);
  }

  /** Using ping to transfer gas to a non-existing contract will lose the gas. */
  @ContractTest(previous = "deploy")
  void retainGasDestinationDoesntExist() {
    final long gasToSend = 1_000_000_000L;
    // Gas for running instructions on the ping contract.
    final long callOverhead = 10L;

    // Save state before ping
    final long userPreAmount = getUserGasBalance(user);
    final long internalPreAmount = getContractGasBalance(pingContractAddress);

    // Call ping functionality (will fail since destination doesn't exist)
    Assertions.assertThatCode(
            () -> pingNoCallback(wrongAddress, gasToSend, gasToSend + callOverhead))
        .isInstanceOf(ActionFailureException.class);

    // Get new state
    final long userPostAmount = getUserGasBalance(user);
    final long internalPostAmount = getContractGasBalance(pingContractAddress);

    // Check that the user has less gas.
    Assertions.assertThat(userPostAmount).isEqualTo(userPreAmount - gasToSend - callOverhead);
    // But the ping contract is (almost) unchanged.
    Assertions.assertThat(internalPostAmount).isEqualTo(internalPreAmount - 1);
  }

  /** Pinging with a cost less than the network fee will fail. */
  @ContractTest(previous = "deploy")
  void pingCostLessThanFee() {
    Assertions.assertThatCode(() -> pingNoCallback(differentContractAddress, NETWORK_FEE - 1))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Allocated cost 129 is less than the network fee 130");
  }

  /**
   * Pinging a non-existing contract with maximum cost correctly reports that the destination
   * doesn't exist.
   */
  @ContractTest(previous = "deploy")
  void pingNonExistentContractMaxNone() {
    Assertions.assertThatCode(() -> ping(wrongAddress, null))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Contract does not exist");
  }

  /**
   * Pinging a non-existing contract with the network fee as cost correctly reports that the
   * destination doesn't exist.
   */
  @ContractTest(previous = "deploy")
  void pingNonExistentContractMaxOne() {
    // 1 is the current test network fee.
    Assertions.assertThatCode(() -> ping(wrongAddress, NETWORK_FEE))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Contract does not exist");
  }

  /** Pinging an existing contract with a cost of 0 will fail. */
  @ContractTest(previous = "deploy")
  void pingNoCost() {
    Assertions.assertThatCode(() -> ping(differentContractAddress, 0L))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Ran out of gas");
  }

  /**
   * Pinging a non-existing contract with a cost less than the network fee will fail, without
   * reporting if the destination exists.
   */
  @ContractTest(previous = "deploy")
  void pingNonExistentContractMaxZero() {
    Assertions.assertThatCode(() -> ping(wrongAddress, 0L))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Ran out of gas");
  }

  private long getUserGasBalance(BlockchainAddress accountAddress) {
    FuzzyState contractJson = blockchain.getAccountPluginAccountJson(accountAddress);
    // Governance -> Account.java
    Unsigned256 value =
        Unsigned256.create(contractJson.getNode("/accountCoins/0/balance").textValue());
    return convertBalanceToGas(value.longValueExact());
  }

  private long getContractGasBalance(BlockchainAddress contractAddress) {
    FuzzyState contractJson = blockchain.getAccountPluginContractJson(contractAddress);
    // Governance -> ContractStorage.java
    Unsigned256 value = Unsigned256.create(contractJson.getNode("/balance/value").textValue());
    return convertBalanceToGas(value.longValueExact());
  }

  private long convertBalanceToGas(long balance) {
    FuzzyState globalJson = blockchain.getAccountPluginGlobalJson();
    // Governance -> AccountStateGlobal.java
    long num = globalJson.getNode("/coins/coins/0/conversionRate/numerator").asLong();
    long denom = globalJson.getNode("/coins/coins/0/conversionRate/numerator").asLong();
    return balance * num / denom;
  }

  private void ping(BlockchainAddress dest, Long pingCost, long callCost) {
    blockchain.sendAction(user, pingContractAddress, PingContract.ping(dest, pingCost), callCost);
  }

  private void ping(BlockchainAddress dest, Long pingCost) {
    ping(dest, pingCost, DEFAULT_CALL_COST);
  }

  private void pingNoCallback(BlockchainAddress dest, Long pingCost, long callCost) {
    blockchain.sendAction(
        user, pingContractAddress, PingContract.pingNoCallback(dest, pingCost), callCost);
  }

  private void pingNoCallback(BlockchainAddress dest, Long pingCost) {
    pingNoCallback(dest, pingCost, DEFAULT_CALL_COST);
  }
}
