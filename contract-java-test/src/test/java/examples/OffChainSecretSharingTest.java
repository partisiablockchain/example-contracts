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
import com.partisiablockchain.language.testenvironment.executionengine.TestExecutionEngine;
import java.math.BigInteger;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.stream.IntStream;
import org.assertj.core.api.Assertions;

/** Test suite for the {@link OffChainSecretSharing} contract. */
public final class OffChainSecretSharingTest extends JunitContractTest {

  /** Contract bytes for the {@link OffChainSecretSharing} contract. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/off_chain_secret_sharing.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/off_chain_secret_sharing_runner"));

  /** Private keys for each of the engines. */
  private static final List<KeyPair> ENGINE_KEYS =
      List.of(20L, 21L, 22L, 23L).stream().map(BigInteger::valueOf).map(KeyPair::new).toList();

  /** Addresses of the engines. */
  private List<BlockchainAddress> engineAddresses;

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

  /** Deploys contracts and sets up execution engines. */
  @ContractTest
  void setup() {
    sender = blockchain.newAccount(senderKey);
    otherSender = blockchain.newAccount(otherSenderKey);
    engineAddresses = new ArrayList<>();
    engines = new ArrayList<>();

    for (final KeyPair engineKey : ENGINE_KEYS) {
      engineAddresses.add(blockchain.newAccount(engineKey));
      engines.add(blockchain.addExecutionEngine(p -> true, engineKey));
    }

    List<OffChainSecretSharing.NodeConfig> nodeConfigs =
        IntStream.range(0, engineAddresses.size())
            .mapToObj(
                i -> new OffChainSecretSharing.NodeConfig(engineAddresses.get(i), "engine" + i))
            .toList();

    byte[] initPayload = OffChainSecretSharing.initialize(nodeConfigs);
    contractAddress = blockchain.deployContract(sender, CONTRACT_BYTES, initPayload);
    contract = new OffChainSecretSharing(getStateClient(), contractAddress);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(0);
    for (int i = 0; i < state.nodes().size(); i++) {
      OffChainSecretSharing.NodeConfig node = state.nodes().get(i);
      assertThat(node.address()).isEqualTo(engineAddresses.get(i));
      assertThat(node.endpoint()).isEqualTo("engine" + i);
    }
  }

  /** A user can register a sharing on the contract. */
  @ContractTest(previous = "setup")
  void registerSharing() {
    byte[] payload = OffChainSecretSharing.registerSharing(SHARING_ID_1);
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
        uploadRequest(senderKey, engineAddresses.get(0), SHARING_ID_1, new byte[] {1, 2, 3});
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(201);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(1);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_1);
    assertThat(sharing.nodesWithCompletedUpload()).isEqualTo(List.of(true, false, false, false));
  }

  /** With a correct signature the user can download shares from an engine. */
  @ContractTest(previous = "sendShareToEngine")
  void getShareFromEngine() {
    final HttpRequestData requestData =
        downloadRequest(senderKey, engineAddresses.get(0), SHARING_ID_1);
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(200);
    assertThat(response.body().data()).isEqualTo(new byte[] {1, 2, 3});
  }

  /** The contract can have multiple different sharings for different people. */
  @ContractTest(previous = "sendShareToEngine")
  void registerAnotherSharing() {
    byte[] payload = OffChainSecretSharing.registerSharing(SHARING_ID_2);
    blockchain.sendAction(otherSender, contractAddress, payload);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(2);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_2);
    assertThat(sharing.owner()).isEqualTo(otherSender);
    assertThat(sharing.nodesWithCompletedUpload()).isEqualTo(List.of(false, false, false, false));
  }

  /** The engines stores the shares for each sharing. */
  @ContractTest(previous = "registerAnotherSharing")
  void nodeCanStoreMultipleSharings() {
    final HttpRequestData requestData =
        uploadRequest(otherSenderKey, engineAddresses.get(0), SHARING_ID_2, new byte[] {1, 2, 3});
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(201);

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().size()).isEqualTo(2);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_2);
    assertThat(sharing.nodesWithCompletedUpload()).isEqualTo(List.of(true, false, false, false));

    assertThat(engines.get(0).getStorage(contractAddress).size()).isEqualTo(2);
  }

  /** Multiple nodes can have different shares store for a specific sharing. */
  @ContractTest(previous = "registerSharing")
  void eachNodeStoresItsOwnSharing() {
    for (int i = 0; i < engines.size(); i++) {
      final HttpRequestData requestData =
          uploadRequest(senderKey, engineAddresses.get(i), SHARING_ID_1, new byte[] {(byte) i});
      final HttpResponseData response =
          engines.get(i).makeHttpRequest(contractAddress, requestData).response();
      assertThat(response.statusCode()).isEqualTo(201);
    }

    OffChainSecretSharing.ContractState state = contract.getState();
    assertThat(state.secretSharings().get(SHARING_ID_1).nodesWithCompletedUpload())
        .isEqualTo(List.of(true, true, true, true));

    for (int i = 0; i < engines.size(); i++) {
      HttpRequestData getSharesRequest =
          downloadRequest(senderKey, engineAddresses.get(i), SHARING_ID_1);
      HttpResponseData response =
          engines.get(i).makeHttpRequest(contractAddress, getSharesRequest).response();
      assertThat(response.body().data()).isEqualTo(new byte[] {(byte) i});
    }
  }

  /** The engine fails with 400 if trying to upload shares multiple times for single sharing. */
  @ContractTest(previous = "sendShareToEngine")
  void alreadySentShares() {
    final HttpRequestData requestData =
        uploadRequest(senderKey, engineAddresses.get(0), SHARING_ID_1, new byte[] {2, 2, 2});
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(400);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Already stored\" }");
  }

  /**
   * The engine fails with 404 if trying to get shares from a node before it has been uploaded to
   * it.
   */
  @ContractTest(previous = "registerSharing")
  void getNotStored() {
    final HttpRequestData requestData =
        downloadRequest(senderKey, engineAddresses.get(0), SHARING_ID_1);
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(404);
    assertThat(response.bodyAsText())
        .isEqualTo("{ \"error\": \"Sharing haven't been stored yet\" }");
  }

  /**
   * The engine fails with 404 if trying to upload shares for a sharing which has not been
   * registered on the contract.
   */
  @ContractTest(previous = "registerSharing")
  void unknownSharing() {
    final HttpRequestData requestData =
        uploadRequest(senderKey, engineAddresses.get(0), SHARING_ID_2, new byte[] {4, 5, 6});
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(404);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unknown sharing\" }");
  }

  /**
   * The node fails with 401 if trying to upload shares with a signature not belonging to the owner.
   */
  @ContractTest(previous = "registerSharing")
  void wrongAuthorizationPut() {
    final HttpRequestData requestData =
        uploadRequest(otherSenderKey, engineAddresses.get(0), SHARING_ID_1, new byte[] {1, 2, 3});
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
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
        downloadRequest(otherSenderKey, engineAddresses.get(0), SHARING_ID_1);
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 401 if trying to download using a signature meant for another node. */
  @ContractTest(previous = "sendShareToEngine")
  void wrongNodeSignature() {
    final HttpRequestData requestData =
        downloadRequest(otherSenderKey, engineAddresses.get(3), SHARING_ID_1);
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 401 if there is no Authorization header. */
  @ContractTest(previous = "sendShareToEngine")
  void noAuthorizationHeader() {
    final Map<String, List<String>> headers = Map.of();
    HttpRequestData requestData =
        new HttpRequestData("GET", "/shares/" + SHARING_ID_1, headers, "");
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 401 if the Authorization header is not prefixed 'secp256k1'. */
  @ContractTest(previous = "sendShareToEngine")
  void wrongAuthPrefix() {
    final Map<String, List<String>> headers = Map.of("Authorization", List.of("otherPrefix "));
    HttpRequestData requestData =
        new HttpRequestData("GET", "/shares/" + SHARING_ID_1, headers, "");
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
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
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(401);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Unauthorized\" }");
  }

  /** The node fails with 404 if the url path is not '/shares/{shareId}'. */
  @ContractTest(previous = "sendShareToEngine")
  void invalidUrl() {
    final HttpRequestData requestData1 = new HttpRequestData("GET", "/unknown/1", Map.of(), "");
    HttpResponseData response1 =
        engines.get(0).makeHttpRequest(contractAddress, requestData1).response();
    assertThat(response1.statusCode()).isEqualTo(404);
    assertThat(response1.bodyAsText()).isEqualTo("{ \"error\": \"Invalid URL or method\" }");

    final HttpRequestData requestData2 =
        new HttpRequestData("GET", "/shares/1/extrapath", Map.of(), "");
    HttpResponseData response2 =
        engines.get(0).makeHttpRequest(contractAddress, requestData2).response();
    assertThat(response2.statusCode()).isEqualTo(404);
    assertThat(response2.bodyAsText()).isEqualTo("{ \"error\": \"Invalid URL or method\" }");
  }

  /** The node fails with 404 if the request method is not GET or PUT. */
  @ContractTest(previous = "sendShareToEngine")
  void invalidMethod() {
    final HttpRequestData requestData = new HttpRequestData("POST", "/shares/1", Map.of(), "");
    HttpResponseData response =
        engines.get(0).makeHttpRequest(contractAddress, requestData).response();
    assertThat(response.statusCode()).isEqualTo(404);
    assertThat(response.bodyAsText()).isEqualTo("{ \"error\": \"Invalid URL or method\" }");
  }

  /** You cannot use the same sharing id multiple times when registering shares. */
  @ContractTest(previous = "registerSharing")
  void registerSharingTwice() {
    byte[] payload = OffChainSecretSharing.registerSharing(SHARING_ID_1);
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

  private static Hash createMessageHash(
      BlockchainAddress engineAddress,
      BlockchainAddress contractAddress,
      String method,
      String uri,
      byte[] data) {
    return Hash.create(
        stream -> {
          engineAddress.write(stream);
          contractAddress.write(stream);
          stream.writeString(method);
          stream.writeString(uri);
          stream.writeDynamicBytes(data);
        });
  }

  private HttpRequestData uploadRequest(
      KeyPair senderKey, BlockchainAddress engineAddress, BigInteger identifier, byte[] share) {

    String method = "PUT";
    String uri = "/shares/" + identifier;
    Hash messageHash = createMessageHash(engineAddress, contractAddress, method, uri, share);
    Signature signature = senderKey.sign(messageHash);

    final Map<String, List<String>> headers =
        Map.of("Authorization", List.of("secp256k1 " + signature.writeAsString()));
    return new HttpRequestData(method, uri, headers, Bytes.fromBytes(share));
  }

  private HttpRequestData downloadRequest(
      KeyPair senderKey, BlockchainAddress engineAddress, BigInteger identifier) {
    String method = "GET";
    String uri = "/shares/" + identifier;
    Hash messageHash =
        createMessageHash(engineAddress, contractAddress, method, uri, new byte[] {});
    Signature signature = senderKey.sign(messageHash);

    final Map<String, List<String>> headers =
        Map.of("Authorization", List.of("secp256k1 " + signature.writeAsString()));
    return new HttpRequestData(method, uri, headers, "");
  }
}
