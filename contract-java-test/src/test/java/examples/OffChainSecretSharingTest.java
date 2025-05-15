package examples;

import static org.assertj.core.api.Assertions.assertThat;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.container.execution.protocol.Bytes;
import com.partisiablockchain.container.execution.protocol.HttpRequestData;
import com.partisiablockchain.container.execution.protocol.HttpResponseData;
import com.partisiablockchain.crypto.Hash;
import com.partisiablockchain.crypto.KeyPair;
import com.partisiablockchain.crypto.Signature;
import com.partisiablockchain.language.abicodegen.OffChainSecretSharing;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.TestBlockchain;
import com.partisiablockchain.language.testenvironment.executionengine.TestExecutionEngine;
import com.secata.stream.SafeDataOutputStream;
import examples.client.SecretShares;
import examples.client.SecretSharingClient;
import java.math.BigInteger;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import org.assertj.core.api.Assertions;

/** Test suite for the {@link OffChainSecretSharing} contract. */
public final class OffChainSecretSharingTest extends JunitContractTest {

  /** Contract bytes for the {@link OffChainSecretSharing} contract. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/off_chain_secret_sharing.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/off_chain_secret_sharing_runner"));

  /** Private keys for each of the engines. */
  public static final List<KeyPair> ENGINE_KEYS =
      List.of(20L, 21L, 22L, 23L).stream().map(BigInteger::valueOf).map(KeyPair::new).toList();

  /** Configurations of the engines. */
  private List<OffChainSecretSharing.NodeConfig> engineConfigs;

  /** Engine test objects. */
  private List<TestExecutionEngine> engines;

  private final KeyPair senderKey = new KeyPair(BigInteger.TWO);
  private BlockchainAddress sender;
  private final KeyPair otherSenderKey = new KeyPair(BigInteger.valueOf(4));
  private BlockchainAddress otherSender;
  private BlockchainAddress contractAddress;
  private OffChainSecretSharing contract;

  private static final BigInteger SHARING_ID_1 = BigInteger.ONE;
  private static final BigInteger SHARING_ID_2 = BigInteger.TWO;

  /** The sharings to upload to engines. */
  private static final List<byte[]> SHARES_WITH_NONCE =
      List.of(
          nonceAndData((byte) 0, new byte[] {1, 2, 3}),
          nonceAndData((byte) 1, new byte[] {4, 5, 6}),
          nonceAndData((byte) 2, new byte[] {7, 8, 9}),
          nonceAndData((byte) 3, new byte[] {10, 11, 12}));

  /** Commitments for each share. */
  private static final List<Hash> SHARE_COMMITMENTS =
      SHARES_WITH_NONCE.stream().map(SecretShares::createShareCommitment).toList();

  /** Deploys contracts and sets up execution engines. */
  @ContractTest
  void setup() {
    sender = blockchain.newAccount(senderKey);
    otherSender = blockchain.newAccount(otherSenderKey);
    engines = createEngines(blockchain);
    engineConfigs = createEngineConfigs(blockchain);

    byte[] initPayload = OffChainSecretSharing.initialize(engineConfigs);
    contractAddress = blockchain.deployContract(sender, CONTRACT_BYTES, initPayload);
    contract = new OffChainSecretSharing(getStateClient(), contractAddress);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(0);
    for (int i = 0; i < state.nodes().size(); i++) {
      assertThat(state.nodes().get(i)).isEqualTo(engineConfigs.get(i));
    }
  }

  /**
   * Create {@link OffChainSecretSharing.NodeConfig} for engines.
   *
   * @param blockchain Blockchain to create {@link OffChainSecretSharing.NodeConfig} in.
   * @return The created {@link OffChainSecretSharing.NodeConfig}.
   */
  public static List<OffChainSecretSharing.NodeConfig> createEngineConfigs(
      TestBlockchain blockchain) {
    return ENGINE_KEYS.stream()
        .map(blockchain::newAccount)
        .map(
            address ->
                new OffChainSecretSharing.NodeConfig(
                    address, "http://%s.example.org".formatted(address.writeAsString())))
        .toList();
  }

  /**
   * Create {@link TestExecutionEngine}.
   *
   * @param blockchain Blockchain to create {@link TestExecutionEngine} in.
   * @return The created {@link TestExecutionEngine}.
   */
  public static List<TestExecutionEngine> createEngines(TestBlockchain blockchain) {
    return ENGINE_KEYS.stream()
        .map(engineKey -> blockchain.addExecutionEngine(p -> true, engineKey))
        .toList();
  }

  /** A user can register a sharing on the contract. */
  @ContractTest(previous = "setup")
  void registerSharing() {
    byte[] payload = OffChainSecretSharing.registerSharing(SHARING_ID_1, SHARE_COMMITMENTS);
    blockchain.sendAction(sender, contractAddress, payload);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(1);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_1);
    assertThat(sharing.owner()).isEqualTo(sender);
    assertThat(sharing.nodesWithCompletedUpload()).isEqualTo(List.of(false, false, false, false));
  }

  /**
   * If a sharing has been registered, the user can upload shares to an engine using the correct
   * signature. The engine will confirm on-chain when it receives a correct share and the contract
   * state keeps track of which engines have confirmed.
   */
  @ContractTest(previous = "registerSharing")
  void sendShareToEngine() {
    final HttpRequestData requestData =
        uploadRequest(senderKey, engineConfigs.get(0), SHARING_ID_1, SHARES_WITH_NONCE.get(0));
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(201);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(1);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_1);
    assertThat(sharing.nodesWithCompletedUpload()).isEqualTo(List.of(true, false, false, false));
  }

  private HttpResponseData makeEngine0Request(final HttpRequestData requestData) {
    final HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    System.out.println("Response %d: %s".formatted(response.statusCode(), response.bodyAsText()));
    return response;
  }

  /** Shares cannot be downloaded without requesting the download on-chain beforehand. */
  @ContractTest(previous = "sendShareToEngine")
  void cannotDownloadShareWithoutRequest() {
    final HttpRequestData requestData =
        downloadRequest(senderKey, engineConfigs.get(0), SHARING_ID_1);
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertFailedDueToDeadline(response);
  }

  /** Share download cannot be requested before the upload has been confirmed. */
  @ContractTest(previous = "sendShareToEngine")
  void cannotRequestDownloadBeforeAllEnginesReceivedTheirShare() {
    Assertions.assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    sender, contractAddress, OffChainSecretSharing.requestDownload(SHARING_ID_1)))
        .hasMessageContaining("Shares haven't been uploaded to all nodes yet");
  }

  /** The contract can have multiple different sharings for different people. */
  @ContractTest(previous = "sendShareToEngine")
  void registerAnotherSharing() {
    byte[] payload = OffChainSecretSharing.registerSharing(SHARING_ID_2, SHARE_COMMITMENTS);
    blockchain.sendAction(otherSender, contractAddress, payload);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(2);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_2);
    assertThat(sharing.owner()).isEqualTo(otherSender);
    assertThat(sharing.nodesWithCompletedUpload()).isEqualTo(List.of(false, false, false, false));
  }

  /** User can upload to another separate sharing. */
  @ContractTest(previous = "registerAnotherSharing")
  void userUploadToAnotherSeparateSharing() {
    final HttpRequestData requestData =
        uploadRequest(otherSenderKey, engineConfigs.get(0), SHARING_ID_2, SHARES_WITH_NONCE.get(0));
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(201);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(2);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_2);
    assertThat(sharing.nodesWithCompletedUpload()).isEqualTo(List.of(true, false, false, false));

    assertThat(engines.get(0).getStorage(contractAddress).size()).isEqualTo(2);
  }

  /** Nodes has different shares for each sharing. */
  @ContractTest(previous = "registerSharing")
  void eachNodeStoresItsOwnSharing() {
    for (int nodeIdx = 0; nodeIdx < engines.size(); nodeIdx++) {
      final HttpRequestData requestData =
          uploadRequest(
              senderKey, engineConfigs.get(nodeIdx), SHARING_ID_1, SHARES_WITH_NONCE.get(nodeIdx));
      final HttpResponseData response =
          engines.get(nodeIdx).makeHttpRequest(contractAddress, requestData).response();
      assertThat(response.statusCode()).isEqualTo(201);
    }

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().get(SHARING_ID_1).nodesWithCompletedUpload())
        .isEqualTo(List.of(true, true, true, true));
  }

  /** Users can request the download of their owned secret shares. */
  @ContractTest(previous = "eachNodeStoresItsOwnSharing")
  void requestShareDownload() {
    blockchain.sendAction(
        sender, contractAddress, OffChainSecretSharing.requestDownload(SHARING_ID_1));

    assertThat(contract.getState().secretSharings().get(SHARING_ID_1).downloadDeadline())
        .isEqualTo(300014L);
    assertThat(blockchain.getBlockProductionTime()).isEqualTo(15);
  }

  /** Users can download secret shares after requesting the download. */
  @ContractTest(previous = "requestShareDownload")
  void usersDownloadAllSharesForSharing() {
    for (int nodeIdx = 0; nodeIdx < engines.size(); nodeIdx++) {
      final HttpRequestData getSharesRequest =
          downloadRequest(senderKey, engineConfigs.get(nodeIdx), SHARING_ID_1);
      final HttpResponseData response =
          engines.get(nodeIdx).makeHttpRequest(contractAddress, getSharesRequest).response();
      assertThat(response.statusCode()).isEqualTo(200);
      assertThat(response.body().data()).isEqualTo(SHARES_WITH_NONCE.get(nodeIdx));
    }
  }

  /** Users cannot download their secret shares after the deadline has passed. */
  @ContractTest(previous = "requestShareDownload")
  void usersCannotDownloadTheirSecretSharesAfterDeadlineHasPassed() {
    blockchain.waitForBlockProductionTime(300015L);
    final HttpRequestData getSharesRequest =
        downloadRequest(senderKey, engineConfigs.get(0), SHARING_ID_1);

    final HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, getSharesRequest).response();
    assertFailedDueToDeadline(response);
  }

  private void assertFailedDueToDeadline(final HttpResponseData response) {
    assertThat(response.statusCode()).isEqualTo(400);
    assertThat(response.bodyAsText())
        .isEqualTo(
            "{ \"error\": \"Download not requested, or download deadline has been passed\" }");
  }

  /** The engine fails with 400 if trying to upload shares multiple times for single sharing. */
  @ContractTest(previous = "sendShareToEngine")
  void sharesAlreadyStored() {
    final HttpRequestData requestData =
        uploadRequest(senderKey, engineConfigs.get(0), SHARING_ID_1, SHARES_WITH_NONCE.get(0));
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(409);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Already stored\" }");
  }

  /** The engine fails when getting share before it has been uploaded. */
  @ContractTest(previous = "registerSharing")
  void engineFailsWhenGettingShareBeforeHasBeenUploaded() {
    final HttpRequestData requestData =
        downloadRequest(senderKey, engineConfigs.get(0), SHARING_ID_1);
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(400);
    assertThat(response.bodyAsText())
        .isEqualTo(
            "{ \"error\": \"Download not requested, or download deadline has been passed\" }");
  }

  /**
   * The engine fails with 404 if trying to upload shares for a sharing which has not been
   * registered on the contract.
   */
  @ContractTest(previous = "registerSharing")
  void unknownSharing() {
    final HttpRequestData requestData =
        uploadRequest(senderKey, engineConfigs.get(0), SHARING_ID_2, SHARES_WITH_NONCE.get(0));
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(404);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unknown sharing\" }");
  }

  /**
   * The node fails with 401 if trying to upload shares with a signature not belonging to the owner.
   */
  @ContractTest(previous = "registerSharing")
  void wrongAuthorizationPut() {
    final HttpRequestData requestData =
        uploadRequest(otherSenderKey, engineConfigs.get(0), SHARING_ID_1, SHARES_WITH_NONCE.get(0));
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /**
   * The node fails with 401 if trying to download shares with a signature not belonging to the
   * owner.
   */
  @ContractTest(previous = "sendShareToEngine")
  void wrongAuthorizationGet() {
    final HttpRequestData requestData =
        downloadRequest(otherSenderKey, engineConfigs.get(0), SHARING_ID_1);
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 401 if trying to download using a signature meant for another node. */
  @ContractTest(previous = "sendShareToEngine")
  void wrongNodeSignature() {
    final HttpRequestData requestData =
        downloadRequest(otherSenderKey, engineConfigs.get(3), SHARING_ID_1);
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 401 if there is no Authorization header. */
  @ContractTest(previous = "sendShareToEngine")
  void noAuthorizationHeader() {
    final Map<String, List<String>> headers = Map.of();
    HttpRequestData requestData =
        new HttpRequestData("GET", "/shares/" + SHARING_ID_1, headers, "");
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 401 if the Authorization header is not prefixed 'secp256k1'. */
  @ContractTest(previous = "sendShareToEngine")
  void wrongAuthPrefix() {
    final Map<String, List<String>> headers = Map.of("Authorization", List.of("otherPrefix "));
    HttpRequestData requestData =
        new HttpRequestData("GET", "/shares/" + SHARING_ID_1, headers, "");
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 401 if the signature provided is not a valid signature. */
  @ContractTest(previous = "sendShareToEngine")
  void invalidSignature() {
    String invalidSignatureHex = "0102030405060708090a0b0c0d0e0d";
    final Map<String, List<String>> headers =
        Map.of("Authorization", List.of("secp256k1 " + invalidSignatureHex));
    HttpRequestData requestData =
        new HttpRequestData("GET", "/shares/" + SHARING_ID_1, headers, "");
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 404 if the url path is not '/shares/{shareId}'. */
  @ContractTest(previous = "sendShareToEngine")
  void invalidUrl() {
    final HttpRequestData requestData1 = new HttpRequestData("GET", "/unknown/1", Map.of(), "");
    final HttpResponseData response1 = makeEngine0Request(requestData1);
    assertThat(response1.statusCode()).isEqualTo(404);
    assertThat(response1.bodyAsText()).isEqualTo("{ \"error\": \"Invalid URL\" }");

    final HttpRequestData requestData2 =
        new HttpRequestData("GET", "/shares/1/extrapath", Map.of(), "");
    final HttpResponseData response2 = makeEngine0Request(requestData2);
    assertThat(response2.statusCode()).isEqualTo(404);
    assertThat(response2.bodyAsText()).isEqualTo("{ \"error\": \"Invalid URL\" }");
  }

  /** The node fails with 404 if the request method is not GET or PUT. */
  @ContractTest(previous = "sendShareToEngine")
  void invalidMethod() {
    final HttpRequestData requestData = new HttpRequestData("POST", "/shares/1", Map.of(), "");
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(405);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Invalid method\" }");
  }

  /** The node fails with 400 if the sharing id is not a valid number. */
  @ContractTest(previous = "sendShareToEngine")
  void malformedSharingId() {
    final HttpRequestData requestData = new HttpRequestData("GET", "/shares/x", Map.of(), "");
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(400);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Malformed request\" }");
  }

  /** Secret sharing ids must be unique. */
  @ContractTest(previous = "registerSharing")
  void registerSharingTwice() {
    byte[] payload = OffChainSecretSharing.registerSharing(SHARING_ID_1, SHARE_COMMITMENTS);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(otherSender, contractAddress, payload))
        .hasMessageContaining("Cannot register sharing with the same identifier");
  }

  /** Only engines are allowed to call register_shared. */
  @ContractTest(previous = "registerSharing")
  void nonEngineRegisterShared() {
    byte[] payload = OffChainSecretSharing.registerShared(SHARING_ID_1);
    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(otherSender, contractAddress, payload))
        .hasMessageContaining("Caller is not one of the engines");
  }

  /** Fail when sending wrong number of commitments for a secret-sharing. */
  @ContractTest(previous = "setup")
  void failWhenSendingWrongNumberCommitmentsForSecretSharing() {
    byte[] payload = OffChainSecretSharing.registerSharing(SHARING_ID_1, List.of());

    Assertions.assertThatCode(() -> blockchain.sendAction(sender, contractAddress, payload))
        .hasMessageContaining("Invalid number of share commitments");
  }

  /** Fail when uploading the wrong share, even if it is "just" the nonce. */
  @ContractTest(previous = "registerSharing")
  void failWhenUploadingWrongShare() {
    final HttpRequestData requestData =
        uploadRequest(
            senderKey,
            engineConfigs.get(0),
            SHARING_ID_1,
            nonceAndData((byte) 9, new byte[] {1, 2, 3}));
    final HttpResponseData response = makeEngine0Request(requestData);
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText())
        .isEqualTo("{ \"error\": \"User uploaded data doesn't match commitment\" }");
  }

  /**
   * Create a share with with a 32-byte nonce prefix (the given byte repeated) and the real data.
   */
  private static byte[] nonceAndData(byte repeatedNonce, byte[] data) {
    return SafeDataOutputStream.serialize(
        s -> {
          for (int i = 0; i < 32; i++) {
            s.writeByte(repeatedNonce);
          }
          s.write(data);
        });
  }

  /**
   * Create a signed share upload request.
   *
   * @param senderKey Key used to sign request. Not nullable.
   * @param engineConfig Configuration of the engine that request is sent to. Not nullable.
   * @param secretSharingId Identifier of the secret sharing. Not nullable.
   * @param share Share to upload. Not nullable.
   * @return Signed request. Not nullable.
   */
  private HttpRequestData uploadRequest(
      KeyPair senderKey,
      OffChainSecretSharing.NodeConfig engineConfig,
      BigInteger secretSharingId,
      byte[] share) {
    assertThat(share).as("Share must have nonce").hasSizeGreaterThan(32);

    final String method = "PUT";
    final Signature signature =
        SecretSharingClient.createSignatureForOffChainHttpRequest(
            senderKey, engineConfig.address(), contractAddress, method, secretSharingId, share);

    final Map<String, List<String>> headers =
        Map.of("Authorization", List.of("secp256k1 " + signature.writeAsString()));
    return new HttpRequestData(
        method, SecretSharingClient.contractUri(secretSharingId), headers, Bytes.fromBytes(share));
  }

  /**
   * Create a signed share download request.
   *
   * @param senderKey Key used to sign request. Not nullable.
   * @param engineConfig Configuration of the engine that request is sent to. Not nullable.
   * @param secretSharingId Identifier of the secret sharing. Not nullable.
   * @return Signed request. Not nullable.
   */
  private HttpRequestData downloadRequest(
      KeyPair senderKey,
      OffChainSecretSharing.NodeConfig engineConfig,
      BigInteger secretSharingId) {
    final String method = "GET";
    final Signature signature =
        SecretSharingClient.createSignatureForOffChainHttpRequest(
            senderKey,
            engineConfig.address(),
            contractAddress,
            method,
            secretSharingId,
            new byte[0]);

    final Map<String, List<String>> headers =
        Map.of("Authorization", List.of("secp256k1 " + signature.writeAsString()));
    return new HttpRequestData(
        method, SecretSharingClient.contractUri(secretSharingId), headers, "");
  }
}
