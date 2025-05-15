package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.crypto.KeyPair;
import com.partisiablockchain.language.abicodegen.OffChainPublishRandomness;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.TestBlockchain;
import com.partisiablockchain.language.testenvironment.executionengine.TestExecutionEngine;
import java.math.BigInteger;
import java.nio.file.Path;
import java.util.List;
import java.util.stream.IntStream;
import org.assertj.core.api.Assertions;
import org.bouncycastle.util.encoders.Hex;

/** Test suite for the {@link OffChainPublishRandomness} contract. */
public final class OffChainPublishRandomnessTest extends JunitContractTest {

  /** Contract bytes for the {@link OffChainPublishRandomness} contract. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/off_chain_publish_randomness.pbc"),
          Path.of(
              "../rust/target/wasm32-unknown-unknown/release/off_chain_publish_randomness_runner"));

  /** Private keys for each of the engines. */
  private static final List<KeyPair> ENGINE_KEYS = OffChainSecretSharingTest.ENGINE_KEYS;

  /** Configurations of the engines. */
  private List<OffChainPublishRandomness.EngineConfig> engineConfigs;

  /** Engine test objects. */
  @SuppressWarnings("UnusedVariable")
  private List<TestExecutionEngine> engines;

  private final KeyPair senderKey = new KeyPair(BigInteger.TWO);
  private BlockchainAddress sender;
  private BlockchainAddress contractAddress;
  private OffChainPublishRandomness contract;

  private static final List<String> INITIAL_RANDOM_DATA_SHARES =
      List.of(
          "fba7bc34d30643feaba539cbae8734959262bf29c69a5b9d233faf27b01b8ea1",
          "f1ea2360a68f57cbb1a8385d64505d4fc0e302f2a1fb1228ba5e7586ab7c6e8f",
          "1122eedd2bf519649f9f3ba741f6e2203760389912d9ec87f3ff226abb59cdc5",
          "06655509ff7d2e31a5a23a39f7be0bdb64e17b61ec39a4128b1fe9c8b6baadb3");

  /**
   * Publish-randomness contract can be deployed. Commit task is automatically created on
   * deployment.
   *
   * <p>Deploys contracts and sets up execution engines.
   */
  @ContractTest
  void setup() {
    sender = blockchain.newAccount(senderKey);
    engineConfigs = createEngineConfigs(blockchain);

    byte[] initPayload = OffChainPublishRandomness.initialize(engineConfigs);
    contractAddress = blockchain.deployContract(sender, CONTRACT_BYTES, initPayload);
    contract = new OffChainPublishRandomness(getStateClient(), contractAddress);

    assertInitialState();
  }

  /** Commit tasks triggers engines commiting to randomness and then later uploading shares. */
  @ContractTest(previous = "setup")
  void enginesSendRandomShares() {
    setupEngines(4);
    assertCommitAndUploadPerformed(1, INITIAL_RANDOM_DATA_SHARES);
  }

  /** Randomness can be consumed, which will trigger creation of new randomness. */
  @ContractTest(previous = "enginesSendRandomShares")
  void randomnessConsumedWhichWillTriggerNewRandomness() {
    blockchain.sendAction(sender, contractAddress, OffChainPublishRandomness.consumeRandomness());

    assertCommitAndUploadPerformed(
        2,
        List.of(
            "cec5d7d0df156f99614ce9ad83f4f5a97d6bf40fb19d994bed6da12e40188bc2",
            "99c1fc8ffec77c121b14cba0b4392080995241cc158fbc83a41c09fbf144d1f0",
            "aedc2821c0714ae132dd1d293a8211e7a53462290cc1d5a3e576f1d5c6bdccc4",
            "d359c9111c92bad3331efd2321c1c3c5f2779d90cd99d062e9bf9cb7f3d2ca0e"));
  }

  /** Contract will maintain exactly one piece of randomness. */
  @ContractTest(previous = "enginesSendRandomShares")
  void enginesWillNotSendRedundantTransactions() {
    pingContract();
    assertCommitAndUploadPerformed(1, INITIAL_RANDOM_DATA_SHARES);
  }

  /** Non-engine cannot upload randomness. */
  @ContractTest(previous = "enginesSendRandomShares")
  void nonEngineCannotUploadRandomness() {
    Assertions.assertThatCode(
            () ->
                blockchain.sendAction(
                    sender,
                    contractAddress,
                    OffChainPublishRandomness.uploadRandomness(1, new byte[0])))
        .hasMessageContaining("Caller is not one of the engines");
  }

  /** Cannot consume randomness if nothing is available. */
  @ContractTest(previous = "setup")
  void cannotConsumeRandomnessIfNothingAvailable() {
    Assertions.assertThatCode(
            () ->
                blockchain.sendAction(
                    sender, contractAddress, OffChainPublishRandomness.consumeRandomness()))
        .hasMessageContaining("No randomness available!");
  }

  /**
   * Engines commit to randomness before uploading it. Contract doesn't continue to upload stage
   * before all engines have uploaded randomness.
   */
  @ContractTest(previous = "setup")
  void contractWaitsForAllEnginesToUploadCommit() {
    setupEngines(3);
    assertPartialCommitments();
    assertNoUploadTasks();
  }

  /** Check that state have been initialized, without any engine work having been done. */
  private void assertInitialState() {
    final OffChainPublishRandomness.ContractState state = contract.getState();

    Assertions.assertThat(state.commitQueue().taskIdOfLastCreated()).isEqualTo(1);
    Assertions.assertThat(state.commitQueue().taskIdOfCurrent()).isEqualTo(1);
    Assertions.assertThat(state.commitQueue().tasks().get(1).id()).isEqualTo(1);
    Assertions.assertThat(state.commitQueue().tasks().get(1).definition()).isNotNull();
    Assertions.assertThat(state.commitQueue().tasks().get(1).completionData())
        .containsExactly(null, null, null, null);

    assertNoUploadTasks();

    for (int i = 0; i < state.engines().size(); i++) {
      Assertions.assertThat(state.engines().get(i)).isEqualTo(engineConfigs.get(i));
    }
  }

  /**
   * Check that engines have created and uploaded randomness.
   *
   * @param taskId Task identifier for which randomness has been uploaded to.
   * @param randomDataShares Hex strings of the various randomness shares.
   */
  private void assertCommitAndUploadPerformed(int taskId, List<String> randomDataShares) {
    final OffChainPublishRandomness.ContractState state = contract.getState();

    Assertions.assertThat(state.commitQueue().taskIdOfLastCreated()).isEqualTo(taskId);
    Assertions.assertThat(state.commitQueue().taskIdOfCurrent()).isEqualTo(taskId);
    Assertions.assertThat(state.commitQueue().tasks().size()).isEqualTo(0);

    Assertions.assertThat(state.uploadQueue().taskIdOfLastCreated()).isEqualTo(taskId);
    Assertions.assertThat(state.uploadQueue().taskIdOfCurrent()).isEqualTo(taskId);
    Assertions.assertThat(state.uploadQueue().tasks().size()).isEqualTo(1);

    Assertions.assertThat(state.uploadQueue().tasks().get(taskId).completionData())
        .hasSize(4)
        .doesNotContainNull();

    Assertions.assertThat(
            state.uploadQueue().tasks().get(taskId).completionData().stream().map(Hex::toHexString))
        .containsExactlyElementsOf(randomDataShares);
  }

  /** Check that only three commitments have been uploaded. */
  private void assertPartialCommitments() {
    final OffChainPublishRandomness.ContractState state = contract.getState();

    Assertions.assertThat(state.commitQueue().taskIdOfLastCreated()).isEqualTo(1);
    Assertions.assertThat(state.commitQueue().taskIdOfCurrent()).isEqualTo(1);
    Assertions.assertThat(state.commitQueue().tasks().size()).isEqualTo(1);

    Assertions.assertThat(
            state.commitQueue().tasks().get(1).completionData().stream()
                .map(OffChainPublishRandomnessTest::safeToString))
        .containsExactly(
            "f432ec4869998609e50c1effd8421b2440dff4ac2b8fe34acd3c78f001fca40b",
            "c778f4fd77f353968730954cdf519eb8d4837cc59fb81b0652aa248cb36633cd",
            "8b5e9bd6424a2aaf13820484c96476192d7142fd6459cd7342ed28ad49217b20",
            null);
  }

  /** Check that no upload tasks have been created yet. */
  private void assertNoUploadTasks() {
    final OffChainPublishRandomness.ContractState state = contract.getState();
    Assertions.assertThat(state.uploadQueue().taskIdOfLastCreated()).isEqualTo(0);
    Assertions.assertThat(state.uploadQueue().taskIdOfCurrent()).isEqualTo(0);
    Assertions.assertThat(state.uploadQueue().tasks().size()).isEqualTo(0);
  }

  private static String safeToString(Object obj) {
    return obj == null ? null : obj.toString();
  }

  /**
   * Create {@link OffChainPublishRandomness.EngineConfig} for engines.
   *
   * @param blockchain Blockchain to create {@link OffChainPublishRandomness.EngineConfig} in.
   * @return The created {@link OffChainPublishRandomness.EngineConfig}.
   */
  public static List<OffChainPublishRandomness.EngineConfig> createEngineConfigs(
      TestBlockchain blockchain) {
    return ENGINE_KEYS.stream()
        .map(blockchain::newAccount)
        .map(
            address ->
                new OffChainPublishRandomness.EngineConfig(
                    address, "http://%s.example.org".formatted(address.writeAsString())))
        .toList();
  }

  /**
   * Create {@link TestExecutionEngine}.
   *
   * @param numEngines Number of engines to create.
   */
  public void setupEngines(int numEngines) {
    engines =
        IntStream.range(0, numEngines)
            .mapToObj(ENGINE_KEYS::get)
            .map(engineKey -> blockchain.addExecutionEngine(contractAddress::equals, engineKey))
            .toList();

    // Ping to ensure that engines notice the contract. This is needed because
    // newly created engines in junit-contract-test are not automatically
    // informed of all existing contracts.
    pingContract();
  }

  private void pingContract() {
    blockchain.sendAction(sender, contractAddress, new byte[0]);
  }
}
