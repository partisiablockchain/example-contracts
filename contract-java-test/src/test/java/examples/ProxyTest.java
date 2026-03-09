package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.container.execution.protocol.HttpRequestData;
import com.partisiablockchain.container.execution.protocol.HttpResponseData;
import com.partisiablockchain.crypto.Hash;
import com.partisiablockchain.crypto.KeyPair;
import com.partisiablockchain.language.abicodegen.Proxy;
import com.partisiablockchain.language.abicodegen.Proxy.EngineConfig;
import com.partisiablockchain.language.abicodegen.Proxy.TransactionPayload;
import com.partisiablockchain.language.abicodegen.Voting;
import com.partisiablockchain.language.abistreams.AbiByteOutput;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.TestBlockchain;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.language.testenvironment.executionengine.TestExecutionEngine;
import com.secata.tools.immutable.Bytes;
import examples.client.ProxyVoteUpload.TransactionRequest;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.stream.Stream;
import org.assertj.core.api.Assertions;

/** Test the {@link Proxy} contract. */
public final class ProxyTest extends JunitContractTest {

  /** {@link Proxy} contract bytes. */
  public static final ContractBytes PROXY_CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/proxy.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/proxy_runner"));

  /** {@link Voting} contract bytes. */
  private static final ContractBytes VOTING_CONTRACT_BYTES = VotingTest.VOTING_CONTRACT_BYTES;

  private static final long VOTING_END_TIME = 60 * 60 * 1000;
  private static final int NUM_ENGINES = 2;

  private BlockchainAddress votingContract;
  private BlockchainAddress voter;

  private BlockchainAddress proxyContract;
  private List<String> authTokens;
  private List<Hash> authHashes;

  private List<KeyPair> engineKeys;
  private List<TestExecutionEngine> engines;
  private List<EngineConfig> engineConfigs;

  /**
   * Given a token string, interprets it as bytes using UTF_8 and returns its hash.
   *
   * @param token The string to hash
   * @return The hash created by interpreting the token string as UTF_8 bytes
   */
  Hash hashToken(String token) {
    return Hash.create(
        s -> {
          s.write(token.getBytes(StandardCharsets.UTF_8));
        });
  }

  /**
   * Create a list of {@link KeyPair}.
   *
   * @param numKeys Number of key pairs to create
   * @return A list of the newly created key pairs
   */
  public static List<KeyPair> createKeyPairs(int numKeys) {
    return Stream.generate(KeyPair::new).limit(numKeys).toList();
  }

  /**
   * Creates {@link TestExecutionEngine}s.
   *
   * @param blockchain Blockchain to create {@link TestExecutionEngine} in
   * @param engineKeys The engine keys to use for creation
   * @return The created {@link TestExecutionEngine}s
   */
  public static List<TestExecutionEngine> createEngines(
      TestBlockchain blockchain, List<KeyPair> engineKeys) {
    return engineKeys.stream()
        .map(engineKey -> blockchain.addExecutionEngine(p -> true, engineKey))
        .toList();
  }

  /**
   * Creates {@link Proxy.EngineConfig}s for engines and registers them with the blockchain.
   *
   * @param blockchain Blockchain to create {@link Proxy.EngineConfig}s in.
   * @param engineKeys The engine keys to use for config creation and blockchain registration.
   * @return The created {@link Proxy.EngineConfig}s.
   */
  public static List<Proxy.EngineConfig> createEngineConfigs(
      TestBlockchain blockchain, List<KeyPair> engineKeys) {
    return engineKeys.stream()
        .map(blockchain::newAccount)
        .map(
            address ->
                new Proxy.EngineConfig(
                    address, "http://%s.example.org".formatted(address.writeAsString())))
        .toList();
  }

  /**
   * Constructs an {@link Proxy.HttpRequestData} with the given arguments.
   *
   * @param method The method type of the request e.g. GET or PUT
   * @param uri The uniform resource identifier of the request
   * @param headers A list of headers associated with the request
   * @param transactionRequest The {@link Proxy.TransactionRequest} to be forwarded
   * @return A corresponding HTTP request whose body contains the serialized transaction payload.
   */
  HttpRequestData createHttpRequest(
      String method,
      String uri,
      Map<String, List<String>> headers,
      TransactionRequest transactionRequest) {
    Bytes transactionBytes =
        Bytes.fromBytes(AbiByteOutput.serializeBigEndian(transactionRequest::serialize));
    return new HttpRequestData(method, uri, headers, transactionBytes);
  }

  /**
   * Retrieves the current state of the `Voting` smart contract.
   *
   * @return Current state of the `Voting` smart contract
   */
  private Voting.VoteState getVotingState() {
    return new Voting(getStateClient(), votingContract).getState();
  }

  /** Proxy contract can be deployed. */
  @ContractTest
  void setUp() {
    voter = blockchain.newAccount(2);

    // Setup engines
    engineKeys = createKeyPairs(NUM_ENGINES);
    engines = createEngines(blockchain, engineKeys);
    engineConfigs = createEngineConfigs(blockchain, engineKeys);

    // Deploy proxy contract with the specified authorization tokens
    authTokens = List.of("token1", "token2", "token3");
    authHashes = authTokens.stream().map(this::hashToken).toList();
    byte[] proxyInitRpc = Proxy.initialize(authHashes, engineConfigs);
    proxyContract = blockchain.deployContract(voter, PROXY_CONTRACT_BYTES, proxyInitRpc);

    // Deploy voting contract
    // Note: the proxy contract, not `voter`, is registered in the list of voters.
    // This is because the proxy contract needs to act as the custodian for all on-chain actions of
    // `voter`.
    byte[] initRpc = Voting.initialize(10, List.of(proxyContract), VOTING_END_TIME);
    votingContract = blockchain.deployContract(voter, VOTING_CONTRACT_BYTES, initRpc);
  }

  /** Authorized requests from assigned engines are forwarded to target contracts. */
  @ContractTest(previous = "setUp")
  void canForwardTransaction() {
    // Check that there are zero votes before
    Voting.VoteState state = getVotingState();
    Assertions.assertThat(state.votes().size()).isEqualTo(0);

    // Create the HTTP request for voting via the proxy contract
    int gasCost = 2_600;
    byte[] rpc = Voting.vote(true);
    TransactionRequest transactionRequest = new TransactionRequest(votingContract, rpc, gasCost);
    HttpRequestData request =
        createHttpRequest(
            "POST",
            "",
            Map.of("Authorization", List.of("Bearer " + authTokens.get(0))),
            transactionRequest);

    // Check that the http request was successfully forwarded
    HttpResponseData response = engines.get(0).makeHttpRequest(proxyContract, request).response();
    Assertions.assertThat(response.statusCode()).isEqualTo(200);
    Assertions.assertThat(new String(response.body().data(), StandardCharsets.UTF_8))
        .isEqualTo("Ok");

    // Check that the voter, via the proxy contract, has voted for "true"
    state = getVotingState();
    Assertions.assertThat(state.votes().size()).isEqualTo(1);
    Assertions.assertThat(getVotingState().votes()).hasSize(1).containsEntry(proxyContract, true);
  }

  /** The off-chain REST endpoint rejects non-POST requests with a 400 error. */
  @ContractTest(previous = "setUp")
  void invalidHttpRequestNotPostRequest() {
    HttpRequestData request =
        createHttpRequest(
            "PUT", "", Map.of(), new TransactionRequest(votingContract, new byte[0], 0));
    HttpResponseData response = engines.get(0).makeHttpRequest(proxyContract, request).response();

    Assertions.assertThat(response.statusCode()).isEqualTo(400);
    Assertions.assertThat(response.bodyAsText()).isEqualTo(String.format("Not a POST request"));
  }

  /**
   * The off-chain REST endpoint rejects requests without authorization headers with a 400 error.
   */
  @ContractTest(previous = "setUp")
  void invalidHttpRequestNoAuthorizationHeader() {
    HttpRequestData request =
        createHttpRequest(
            "POST", "", Map.of(), new TransactionRequest(votingContract, new byte[0], 0));
    HttpResponseData response = engines.get(0).makeHttpRequest(proxyContract, request).response();

    Assertions.assertThat(response.statusCode()).isEqualTo(400);
    Assertions.assertThat(response.bodyAsText())
        .isEqualTo(String.format("No \"Authorization\" header provided"));
  }

  /**
   * The off-chain REST endpoint rejects requests with invalid authorization header formats with a
   * 400 error.
   */
  @ContractTest(previous = "setUp")
  void invalidHttpRequestBadAuthorizationHeader() {
    HttpRequestData request =
        createHttpRequest(
            "POST",
            "",
            Map.of("Authorization", List.of("Bad authorization field name")),
            new TransactionRequest(votingContract, new byte[0], 0));
    HttpResponseData response = engines.get(0).makeHttpRequest(proxyContract, request).response();

    Assertions.assertThat(response.statusCode()).isEqualTo(400);
    Assertions.assertThat(response.bodyAsText())
        .isEqualTo(
            String.format(
                "The \"Authorization\" header field must be formatted as \"Authorization: Bearer"
                    + " <TOKEN>\""));
  }

  /** The off-chain REST endpoint rejects requests with unauthorized tokens with a 401 error. */
  @ContractTest(previous = "setUp")
  void invalidHttpRequestUnauthorizedToken() {
    HttpRequestData request =
        createHttpRequest(
            "POST",
            "",
            Map.of("Authorization", List.of("Bearer " + "Unauthorized Token")),
            new TransactionRequest(votingContract, new byte[0], 0));
    HttpResponseData response = engines.get(0).makeHttpRequest(proxyContract, request).response();

    Assertions.assertThat(response.statusCode()).isEqualTo(401);
    Assertions.assertThat(response.bodyAsText()).isEqualTo(String.format("Token is unauthorized"));
  }

  /** Transactions from unassigned engines are rejected. */
  @ContractTest(previous = "setUp")
  void unassignedExecutionEngineFails() {
    // Create a valid transaction payload
    byte[] rpc = Voting.vote(true);
    TransactionPayload transactionPayload = new TransactionPayload(votingContract, rpc);

    byte[] forwardRpc = Proxy.forward(transactionPayload);

    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter, proxyContract, forwardRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            String.format(
                "The address %s is not a known execution engine address",
                voter.writeAsString().toUpperCase(Locale.getDefault())));
  }
}
