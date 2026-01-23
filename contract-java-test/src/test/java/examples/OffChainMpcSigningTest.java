package examples;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.container.execution.protocol.storage.StorageKey;
import com.partisiablockchain.crypto.BlockchainPublicKey;
import com.partisiablockchain.crypto.Hash;
import com.partisiablockchain.crypto.KeyPair;
import com.partisiablockchain.crypto.Signature;
import com.partisiablockchain.language.abicodegen.OffChainMpcSigning;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.TestBlockchain;
import com.partisiablockchain.language.testenvironment.TxExecution;
import com.partisiablockchain.language.testenvironment.executionengine.TestExecutionEngine;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.stream.IntStream;
import java.util.stream.Stream;

/** Test suite for the {@link OffChainMpcSigning} contract. */
public final class OffChainMpcSigningTest extends JunitContractTest {

  public static BigInteger CURVE_ORDER =
      new BigInteger("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141", 16);

  /** Contract bytes for the {@link OffChainMpcSigning} contract. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytesLoader.forContract("off_chain_mpc_signing");

  /** Private keys for each of the engines. */
  private static final List<KeyPair> ENGINE_KEYS =
      Stream.of(20L, 21L, 22L).map(BigInteger::valueOf).map(KeyPair::new).toList();

  /** Configurations of the engines. */
  private List<OffChainMpcSigning.EngineConfig> engineConfigs;

  /** Engine test objects. */
  private List<TestExecutionEngine> engines;

  private final KeyPair ownerKey = new KeyPair(BigInteger.TWO);
  private final KeyPair senderKey = new KeyPair(BigInteger.valueOf(80));
  private BlockchainAddress owner;
  private BlockchainAddress sender;
  private BlockchainAddress contractAddress;

  /**
   * Can deploy contract.
   *
   * <p>Deploys contracts and sets up execution engines.
   */
  @ContractTest
  void setupWithoutEngines() {
    deployContract(2, 5);
  }

  /**
   * Engines generates the signing key and starts preprocessing immediately when contract is
   * deployed.
   */
  @ContractTest(previous = "setupWithoutEngines")
  void setup() {
    setupEngines(3);

    assertGeneratedSigningKey();
    assertThat(
            getState()
                .signingComputationState()
                .preprocessState()
                .createdOrQueuedPreprocessMaterial())
        .isEqualTo(5);
  }

  /** Can deploy the contract without doing any preprocessing. */
  @ContractTest
  void setupWithoutPreprocessing() {
    deployContract(0, 1);
    setupEngines(3);
    assertGeneratedSigningKey();
    assertThat(
            getState()
                .signingComputationState()
                .preprocessState()
                .createdOrQueuedPreprocessMaterial())
        .isEqualTo(0);
  }

  /** Can sign a message. */
  @ContractTest(previous = "setup")
  void sign() {
    byte[] message = "hello world".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));

    assertThat(getState().signingComputationState().signingInformation().get(1).requestingAddress())
        .isEqualTo(sender);
    verifyValidSignature(1, message);
  }

  /**
   * If multiple sign requests are ready to be processed, the engine batches the completions into a
   * single signed transaction, (maximum 10 at once).
   */
  @ContractTest
  void signMultipleBatched() {
    deployContract(1, 12);
    setupEngines(3);
    engines.get(1).stop();
    engines.get(2).stop();
    for (int i = 1; i < 12; i++) {
      byte[] message = ("hello world " + i).getBytes(StandardCharsets.UTF_8);
      blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));
    }
    List<TxExecution> txExecutions = engines.get(1).runOffChainStateChange(contractAddress);
    assertThat(txExecutions).hasSize(1);
    assertThat(txExecutions.get(0).getSpawnedEvents().get(0).getSpawnedEvents()).hasSize(10);
    for (int i = 1; i < 11; i++) {
      byte[] message = ("hello world " + i).getBytes(StandardCharsets.UTF_8);
      assertThat(
              getState().signingComputationState().signingInformation().get(i).requestingAddress())
          .isEqualTo(sender);
      verifyValidSignature(i, message);
    }
  }

  /** The contract can sign even if only two of the engines are alive. */
  @ContractTest(previous = "setup")
  void signOnlyTwoEngines() {
    for (int i = 0; i < engines.size(); i++) {
      engines.get(i).stop();
      int signingId = i + 1;
      byte[] message = "Sign with two engines".getBytes(StandardCharsets.UTF_8);
      blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));

      assertThat(
              getState()
                  .signingComputationState()
                  .signingInformation()
                  .get(signingId)
                  .requestingAddress())
          .isEqualTo(sender);
      verifyValidSignature(signingId, message);
      engines.get(i).resume();
    }
  }

  /**
   * If a node sends shares that cannot be opened, then the contract waits for the third input
   * before creating the signature.
   */
  @ContractTest(previous = "setup")
  void signNodeSendsIncorrectShareUnableToOpen() {
    engines.get(1).stop();
    engines.get(2).stop();
    byte[] message = "Sign with incorrect share".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));

    int signingId = 1;
    blockchain.sendAction(
        engineConfigs.get(1).address(),
        contractAddress,
        OffChainMpcSigning.signReport(
            List.of(
                new OffChainMpcSigning.WithIdTaskSignCompletion(
                    signingId,
                    new OffChainMpcSigning.TaskSignCompletion(
                        new OffChainMpcSigning.ReplicatedSecretShareU256(
                            List.of(BigInteger.ZERO, BigInteger.ZERO)),
                        new OffChainMpcSigning.ReplicatedSecretShareU256(
                            List.of(BigInteger.ZERO, BigInteger.ZERO)))))));

    OffChainMpcSigning.ContractState state = getState();
    assertThat(
            state
                .signingComputationState()
                .signQueue()
                .tasks()
                .get(signingId)
                .completionData()
                .get(1))
        .isNotNull();
    assertThat(state.signingComputationState().signingInformation().get(signingId).signature())
        .isEqualTo(null);

    engines.get(2).resume();
    verifyValidSignature(signingId, message);
  }

  /**
   * If a node sends shares that produce a signature that cannot be verified, then the contract
   * waits for the third input before creating the signature.
   */
  @ContractTest(previous = "setup")
  void signNodeSendsIncorrectShareUnableToVerify() {
    engines.get(1).stop();
    engines.get(2).stop();
    byte[] message = "Sign with incorrect share".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));

    int signingId = 1;
    OffChainMpcSigning.ContractState state = getState();
    OffChainMpcSigning.TaskSignCompletion node0TaskSignCompletion =
        state.signingComputationState().signQueue().tasks().get(signingId).completionData().get(0);
    blockchain.sendAction(
        engineConfigs.get(2).address(),
        contractAddress,
        OffChainMpcSigning.signReport(
            List.of(
                new OffChainMpcSigning.WithIdTaskSignCompletion(
                    signingId,
                    new OffChainMpcSigning.TaskSignCompletion(
                        new OffChainMpcSigning.ReplicatedSecretShareU256(
                            List.of(
                                node0TaskSignCompletion.sPrime().shares().get(1), BigInteger.ZERO)),
                        new OffChainMpcSigning.ReplicatedSecretShareU256(
                            List.of(
                                node0TaskSignCompletion.gammaPrime().shares().get(1),
                                BigInteger.ZERO)))))));

    state = getState();
    assertThat(
            state
                .signingComputationState()
                .signQueue()
                .tasks()
                .get(signingId)
                .completionData()
                .get(2))
        .isNotNull();
    assertThat(state.signingComputationState().signingInformation().get(signingId).signature())
        .isEqualTo(null);

    engines.get(1).resume();
    verifyValidSignature(signingId, message);
  }

  /**
   * The on-chain transactions fails if a node sends wrong opening shares during preprocessing prep.
   */
  @ContractTest(previous = "setupWithoutPreprocessing")
  void failToOpenPreprocessingPrep() {
    engines.get(2).stop();
    byte[] message = "Sign to start preprocessing".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));
    int signingId = 1;
    BigInteger zero = BigInteger.ZERO;

    engines.get(2).runOffChainStateChange(contractAddress);
    byte[] preReportRpc =
        OffChainMpcSigning.prepReport(
            signingId,
            new OffChainMpcSigning.TaskPrepCompletion(
                List.of(zero),
                List.of(zero),
                List.of(zero),
                List.of(zero),
                List.of(
                    new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
                        List.of(
                            new OffChainMpcSigning.EncodedCurvePoint(new byte[33]),
                            new OffChainMpcSigning.EncodedCurvePoint(new byte[33]))))));
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(2).address(), contractAddress, preReportRpc))
        .hasMessageContaining("Unable to open R");
  }

  /**
   * The on-chain transactions fails if a node sends wrong opening shares during preprocessing mul
   * check.
   */
  @ContractTest(previous = "setupWithoutPreprocessing")
  void failToOpenPreprocessingMulCheckRand() {
    engines.get(2).stop();
    byte[] message = "Sign to start preprocessing".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));
    int signingId = 1;
    BigInteger zero = BigInteger.ZERO;

    engines.get(2).runOffChainStateChange(contractAddress);
    engines.get(2).runOffChainStateChange(contractAddress);
    byte[] wrongRandomnessRpc =
        OffChainMpcSigning.mulCheckOneReport(
            signingId,
            new OffChainMpcSigning.TaskMulCheckOneCompletion(
                new OffChainMpcSigning.ReplicatedSecretShareU256(List.of(zero, zero)),
                new OffChainMpcSigning.ReplicatedSecretShareU256(List.of(zero, zero))));
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(2).address(), contractAddress, wrongRandomnessRpc))
        .hasMessageContaining("Unable to open mul check randomness");
  }

  /**
   * The on-chain transactions fails if a node sends wrong opening shares during preprocessing mul
   * check.
   */
  @ContractTest(previous = "setupWithoutPreprocessing")
  void failToOpenPreprocessingMulCheckEpsilon() {
    engines.get(2).stop();
    byte[] message = "Sign to start preprocessing".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));
    int signingId = 1;
    BigInteger zero = BigInteger.ZERO;

    engines.get(2).runOffChainStateChange(contractAddress);
    engines.get(2).runOffChainStateChange(contractAddress);

    List<OffChainMpcSigning.TaskMulCheckOneCompletion> inputs =
        getState()
            .signingComputationState()
            .mulCheckOneQueue()
            .tasks()
            .get(signingId)
            .completionData();
    byte[] wrongEpsilonRpc =
        OffChainMpcSigning.mulCheckOneReport(
            signingId,
            new OffChainMpcSigning.TaskMulCheckOneCompletion(
                new OffChainMpcSigning.ReplicatedSecretShareU256(
                    List.of(
                        inputs.get(0).rand().shares().get(1),
                        inputs.get(1).rand().shares().get(0))),
                new OffChainMpcSigning.ReplicatedSecretShareU256(List.of(zero, zero))));
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(2).address(), contractAddress, wrongEpsilonRpc))
        .hasMessageContaining("Unable to open epsilon");
  }

  /** The on-chain transactions fails if the nodes fail the multiplication check. */
  @ContractTest(previous = "setupWithoutPreprocessing")
  void failsMulCheck() {
    engines.get(2).stop();
    byte[] message = "Sign to start preprocessing".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));
    int signingId = 1;

    engines.get(2).runOffChainStateChange(contractAddress);
    engines.get(2).runOffChainStateChange(contractAddress);
    engines.get(2).runOffChainStateChange(contractAddress);

    byte[] mulCheckTwoReportRpc =
        OffChainMpcSigning.mulCheckTwoReport(
            signingId, new OffChainMpcSigning.TaskMulCheckTwoCompletion(BigInteger.ZERO));

    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(2).address(), contractAddress, mulCheckTwoReportRpc))
        .hasMessageContaining("Malicious activity detected. Failed the mul check.");
  }

  /** Can reset preprocessing, which prompts the engines to start preprocessing again. */
  @ContractTest(previous = "setup")
  void resetPreprocessing() {
    OffChainMpcSigning.ContractState state = getState();
    assertThat(state.signingComputationState().preprocessState().allocatedPreprocessMaterial())
        .isEqualTo(0);
    assertThat(
            state.signingComputationState().preprocessState().createdOrQueuedPreprocessMaterial())
        .isEqualTo(5);

    blockchain.sendAction(owner, contractAddress, OffChainMpcSigning.resetPreprocessing());

    state = getState();
    assertThat(state.signingComputationState().preprocessState().allocatedPreprocessMaterial())
        .isEqualTo(5);
    assertThat(
            state.signingComputationState().preprocessState().createdOrQueuedPreprocessMaterial())
        .isEqualTo(10);

    byte[] message = "after reset".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));
    verifyValidSignature(1, message);
    state = getState();
    assertThat(
            state.signingComputationState().signQueue().tasks().get(1).definition().preprocessId())
        .isEqualTo(6);
  }

  /**
   * If there are queues not yet finished when resetting preprocessing, those queues are canceled.
   */
  @ContractTest(previous = "setupWithoutPreprocessing")
  void resetPreprocessingCancelsQueues() {
    engines.get(0).stop();
    blockchain.sendAction(
        sender,
        contractAddress,
        OffChainMpcSigning.signMessage(
            "not succeeding message 1".getBytes(StandardCharsets.UTF_8)));
    blockchain.sendAction(
        sender,
        contractAddress,
        OffChainMpcSigning.signMessage(
            "not succeeding message 2".getBytes(StandardCharsets.UTF_8)));
    OffChainMpcSigning.ContractState state = getState();
    assertThat(state.signingComputationState().signQueue().queue().size()).isEqualTo(2);
    assertThat(state.signingComputationState().prePrepCheckQueue().queue().size()).isEqualTo(2);
    assertThat(state.signingComputationState().preprocessState().allocatedPreprocessMaterial())
        .isEqualTo(2);

    blockchain.sendAction(owner, contractAddress, OffChainMpcSigning.resetPreprocessing());

    state = getState();
    assertThat(state.signingComputationState().signQueue().queue().size()).isEqualTo(0);
    assertThat(state.signingComputationState().prePrepCheckQueue().queue().size()).isEqualTo(0);
    assertThat(state.signingComputationState().preprocessState().allocatedPreprocessMaterial())
        .isEqualTo(2);

    engines.get(0).resume();

    byte[] message = "after reset".getBytes(StandardCharsets.UTF_8);
    blockchain.sendAction(sender, contractAddress, OffChainMpcSigning.signMessage(message));
    verifyValidSignature(3, message);
    state = getState();
    assertThat(
            state.signingComputationState().signQueue().tasks().get(3).definition().preprocessId())
        .isEqualTo(3);
  }

  /** Only the owner of the contract can reset preprocessing. */
  @ContractTest(previous = "setup")
  void onlyOwnerCanReset() {
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    sender, contractAddress, OffChainMpcSigning.resetPreprocessing()))
        .hasMessageContaining("Only the owner can reset the contract");
  }

  /** The contract must be initialized with three nodes. */
  @ContractTest
  void wrongNumberNodes() {
    sender = blockchain.newAccount(senderKey);
    engineConfigs = createEngineConfigs(blockchain);

    byte[] initPayload =
        OffChainMpcSigning.initialize(
            engineConfigs.subList(0, 2), new OffChainMpcSigning.PreprocessConfig(0, 1));
    assertThatThrownBy(() -> blockchain.deployContract(sender, CONTRACT_BYTES, initPayload))
        .hasMessageContaining("Expected 3 engines. Got 2.");
  }

  /** The transaction fails if an engine sends an incorrect share of the public key. */
  @ContractTest(previous = "setupWithoutEngines")
  void cannotOpenPubKey() {
    for (KeyPair engineKey : ENGINE_KEYS) {
      BlockchainPublicKey pubKey = engineKey.getPublic();
      blockchain.sendAction(
          pubKey.createAddress(),
          contractAddress,
          OffChainMpcSigning.uploadEnginePubKey(1, pubKey));
    }

    OffChainMpcSigning.EncodedCurvePoint publicKeyShare1 =
        new OffChainMpcSigning.EncodedCurvePoint(
            new KeyPair(BigInteger.valueOf(7)).getPublic().asBytes());
    OffChainMpcSigning.EncodedCurvePoint publicKeyShare2 =
        new OffChainMpcSigning.EncodedCurvePoint(
            new KeyPair(BigInteger.valueOf(8)).getPublic().asBytes());
    OffChainMpcSigning.EncodedCurvePoint publicKeyShare3 =
        new OffChainMpcSigning.EncodedCurvePoint(
            new KeyPair(BigInteger.valueOf(9)).getPublic().asBytes());
    OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint replicatedShare1 =
        new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
            List.of(publicKeyShare3, publicKeyShare2));
    OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint replicatedShare2 =
        new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
            List.of(publicKeyShare1, publicKeyShare3));
    OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint replicatedShare3 =
        new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
            List.of(publicKeyShare2, publicKeyShare1));

    OffChainMpcSigning.EncodedCurvePoint incorrectShare =
        new OffChainMpcSigning.EncodedCurvePoint(
            new KeyPair(BigInteger.valueOf(10)).getPublic().asBytes());

    assertThatThrownBy(
            () ->
                enginesSendSigningPublicKey(
                    List.of(
                        new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
                            List.of(incorrectShare, publicKeyShare2)),
                        replicatedShare2,
                        replicatedShare3)))
        .hasMessageContaining("Unable to open public key");
    assertThatThrownBy(
            () ->
                enginesSendSigningPublicKey(
                    List.of(
                        replicatedShare1,
                        new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
                            List.of(incorrectShare, publicKeyShare3)),
                        replicatedShare3)))
        .hasMessageContaining("Unable to open public key");
    assertThatThrownBy(
            () ->
                enginesSendSigningPublicKey(
                    List.of(
                        replicatedShare1,
                        replicatedShare2,
                        new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
                            List.of(incorrectShare, publicKeyShare1)))))
        .hasMessageContaining("Unable to open public key");
  }

  /** Cannot sign a message before the engines have generated the public key. */
  @ContractTest(previous = "setupWithoutEngines")
  void cantSignBeforePublicKey() {
    byte[] message = "message before public key".getBytes(StandardCharsets.UTF_8);
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    sender, contractAddress, OffChainMpcSigning.signMessage(message)))
        .hasMessageContaining("Unable to sign. Engines haven't finished generating the key.");
  }

  /** If an engine sends an invocation to an already completed task it will fail. */
  @ContractTest(previous = "setup")
  void cantSendCompletionForFinishedTasks() {
    int taskId = 1;
    byte[] prePrepRpc =
        OffChainMpcSigning.prePrepCheckReport(
            taskId, new OffChainMpcSigning.TaskPrePrepCheckCompletion(List.of()));
    assertThatThrownBy(
            () ->
                blockchain.sendAction(engineConfigs.get(0).address(), contractAddress, prePrepRpc))
        .hasMessageContaining("Task 1 is not an active task");
    byte[] prepRpc =
        OffChainMpcSigning.prepReport(
            taskId,
            new OffChainMpcSigning.TaskPrepCompletion(
                List.of(), List.of(), List.of(), List.of(), List.of()));
    assertThatThrownBy(
            () -> blockchain.sendAction(engineConfigs.get(0).address(), contractAddress, prepRpc))
        .hasMessageContaining("Task 1 is not an active task");
    byte[] mulCheckOneRpc =
        OffChainMpcSigning.mulCheckOneReport(
            taskId,
            new OffChainMpcSigning.TaskMulCheckOneCompletion(
                new OffChainMpcSigning.ReplicatedSecretShareU256(
                    List.of(BigInteger.ZERO, BigInteger.ZERO)),
                new OffChainMpcSigning.ReplicatedSecretShareU256(
                    List.of(BigInteger.ZERO, BigInteger.ZERO))));
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(0).address(), contractAddress, mulCheckOneRpc))
        .hasMessageContaining("Task 1 is not an active task");
    byte[] mulCheckTwoRpc =
        OffChainMpcSigning.mulCheckTwoReport(
            taskId, new OffChainMpcSigning.TaskMulCheckTwoCompletion(BigInteger.ZERO));
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(0).address(), contractAddress, mulCheckTwoRpc))
        .hasMessageContaining("Task 1 is not an active task");
  }

  /** Only engines are allowed to report work queue completions. */
  @ContractTest(previous = "setupWithoutEngines")
  void nonEngineSender() {
    int taskId = 1;
    BlockchainPublicKey pubKey = new KeyPair(BigInteger.valueOf(6)).getPublic();
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    sender, contractAddress, OffChainMpcSigning.uploadEnginePubKey(taskId, pubKey)))
        .hasMessageContaining("Caller is not one of the engines");
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    sender,
                    contractAddress,
                    OffChainMpcSigning.uploadPubKeyShare(
                        taskId,
                        new OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint(
                            List.of(
                                new OffChainMpcSigning.EncodedCurvePoint(pubKey.asBytes()),
                                new OffChainMpcSigning.EncodedCurvePoint(pubKey.asBytes()))))))
        .hasMessageContaining("Caller is not one of the engines");
    byte[] prePrepRpc =
        OffChainMpcSigning.prePrepCheckReport(
            taskId, new OffChainMpcSigning.TaskPrePrepCheckCompletion(List.of()));
    assertThatThrownBy(() -> blockchain.sendAction(sender, contractAddress, prePrepRpc))
        .hasMessageContaining("Caller is not one of the engines");
    byte[] prepRpc =
        OffChainMpcSigning.prepReport(
            taskId,
            new OffChainMpcSigning.TaskPrepCompletion(
                List.of(), List.of(), List.of(), List.of(), List.of()));
    assertThatThrownBy(() -> blockchain.sendAction(sender, contractAddress, prepRpc))
        .hasMessageContaining("Caller is not one of the engines");
    byte[] mulCheckOneRpc =
        OffChainMpcSigning.mulCheckOneReport(
            taskId,
            new OffChainMpcSigning.TaskMulCheckOneCompletion(
                new OffChainMpcSigning.ReplicatedSecretShareU256(
                    List.of(BigInteger.ZERO, BigInteger.ZERO)),
                new OffChainMpcSigning.ReplicatedSecretShareU256(
                    List.of(BigInteger.ZERO, BigInteger.ZERO))));
    assertThatThrownBy(() -> blockchain.sendAction(sender, contractAddress, mulCheckOneRpc))
        .hasMessageContaining("Caller is not one of the engines");
    byte[] mulCheckTwoRpc =
        OffChainMpcSigning.mulCheckTwoReport(
            taskId, new OffChainMpcSigning.TaskMulCheckTwoCompletion(BigInteger.ZERO));
    assertThatThrownBy(() -> blockchain.sendAction(sender, contractAddress, mulCheckTwoRpc))
        .hasMessageContaining("Caller is not one of the engines");
  }

  /**
   * The invocation called by the contract for completing the engine work can only be called by the
   * contract.
   */
  @ContractTest(previous = "setup")
  void onlySelfInvocationAllowed() {
    int taskId = 1;
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(0).address(),
                    contractAddress,
                    OffChainMpcSigning.prePrepCheckComplete(taskId)))
        .hasMessageContaining("Can only be called by the contract itself");
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(0).address(),
                    contractAddress,
                    OffChainMpcSigning.prepComplete(taskId)))
        .hasMessageContaining("Can only be called by the contract itself");
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(0).address(),
                    contractAddress,
                    OffChainMpcSigning.mulCheckOneComplete(taskId)))
        .hasMessageContaining("Can only be called by the contract itself");
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(0).address(),
                    contractAddress,
                    OffChainMpcSigning.mulCheckTwoComplete(taskId)))
        .hasMessageContaining("Can only be called by the contract itself");
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    engineConfigs.get(0).address(),
                    contractAddress,
                    OffChainMpcSigning.signComplete(taskId)))
        .hasMessageContaining("Can only be called by the contract itself");
  }

  private void deployContract(int numToPreprocess, int batchSize) {
    owner = blockchain.newAccount(ownerKey);
    sender = blockchain.newAccount(senderKey);
    engineConfigs = createEngineConfigs(blockchain);

    byte[] initPayload =
        OffChainMpcSigning.initialize(
            engineConfigs, new OffChainMpcSigning.PreprocessConfig(numToPreprocess, batchSize));
    contractAddress = blockchain.deployContract(owner, CONTRACT_BYTES, initPayload);
  }

  private void verifyValidSignature(int signingId, byte[] message) {
    OffChainMpcSigning.ContractState state = getState();

    OffChainMpcSigning.SigningInformation signingInformation =
        state.signingComputationState().signingInformation().get(signingId);

    Signature signature = signingInformation.signature();
    assertThat(signature).isNotNull();

    Hash messageHash = Hash.create(stream -> stream.write(message));
    BlockchainPublicKey publicKey = state.signingComputationState().publicKey();

    assertThat(signature.recoverPublicKey(messageHash)).isEqualTo(publicKey);
  }

  private void enginesSendSigningPublicKey(
      List<OffChainMpcSigning.ReplicatedSecretShareEncodedCurvePoint> replicatedShares) {
    for (int i = 0; i < ENGINE_KEYS.size(); i++) {
      blockchain.sendAction(
          ENGINE_KEYS.get(i).getPublic().createAddress(),
          contractAddress,
          OffChainMpcSigning.uploadPubKeyShare(1, replicatedShares.get(i)));
    }
  }

  private void assertGeneratedSigningKey() {
    OffChainMpcSigning.ContractState state = getState();
    assertThat(state.signingComputationState().publicKey()).isNotNull();

    OffChainMpcSigning.ReplicatedSecretShareU256 secretShare0 = getSecretFromEngineStorage(0);
    OffChainMpcSigning.ReplicatedSecretShareU256 secretShare1 = getSecretFromEngineStorage(1);
    OffChainMpcSigning.ReplicatedSecretShareU256 secretShare2 = getSecretFromEngineStorage(2);

    assertThat(secretShare0.shares().get(0)).isEqualTo(secretShare1.shares().get(1));
    assertThat(secretShare1.shares().get(0)).isEqualTo(secretShare2.shares().get(1));
    assertThat(secretShare2.shares().get(0)).isEqualTo(secretShare0.shares().get(1));

    BigInteger openedSecret =
        secretShare0
            .shares()
            .get(0)
            .add(secretShare1.shares().get(0))
            .add(secretShare2.shares().get(0))
            .mod(CURVE_ORDER);

    assertThat(new KeyPair(openedSecret).getPublic())
        .isEqualTo(state.signingComputationState().publicKey());
  }

  /**
   * Create {@link OffChainMpcSigning.EngineConfig} for engines.
   *
   * @param blockchain Blockchain to create {@link OffChainMpcSigning.EngineConfig} in.
   * @return The created {@link OffChainMpcSigning.EngineConfig}.
   */
  public static List<OffChainMpcSigning.EngineConfig> createEngineConfigs(
      TestBlockchain blockchain) {
    return ENGINE_KEYS.stream()
        .map(blockchain::newAccount)
        .map(OffChainMpcSigning.EngineConfig::new)
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

  private OffChainMpcSigning.ReplicatedSecretShareU256 getSecretFromEngineStorage(int engineIndex) {
    byte[] secretKeyBytes =
        engines
            .get(engineIndex)
            .getStorage(contractAddress)
            .get(
                new StorageKey("SIGNING_SECRET_KEY".getBytes(StandardCharsets.UTF_8), new byte[0]));
    return new OffChainMpcSigning.ReplicatedSecretShareU256(
        List.of(
            new BigInteger(1, secretKeyBytes, 0, 32), new BigInteger(1, secretKeyBytes, 32, 32)));
  }

  private OffChainMpcSigning.ContractState getState() {
    return new OffChainMpcSigning(getStateClient(), contractAddress).getState();
  }
}
