package examples.client;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.api.transactionclient.BlockchainTransactionClient;
import com.partisiablockchain.api.transactionclient.SenderAuthenticationKeyPair;
import com.partisiablockchain.api.transactionclient.SentTransaction;
import com.partisiablockchain.api.transactionclient.Transaction;
import com.partisiablockchain.crypto.Hash;
import com.partisiablockchain.crypto.KeyPair;
import com.partisiablockchain.crypto.Signature;
import com.partisiablockchain.language.abicodegen.OffChainSecretSharing;
import com.partisiablockchain.language.codegenlib.BlockchainStateClientImpl;
import com.secata.stream.SafeDataOutputStream;
import com.secata.tools.coverage.ExceptionConverter;
import com.secata.tools.rest.ObjectMapperProvider;
import jakarta.ws.rs.client.Client;
import jakarta.ws.rs.client.ClientBuilder;
import jakarta.ws.rs.client.Entity;
import jakarta.ws.rs.core.MediaType;
import java.math.BigInteger;
import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;
import org.glassfish.jersey.jackson.JacksonFeature;
import org.glassfish.jersey.server.ResourceConfig;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Contract-specific client for interacting with a {@code off-chain-secret-sharing} contract.
 *
 * <p>This client implements require login for {@link #registerAndUploadSharing uploading} and
 * {@link #downloadAndReconstruct downloading} shares from the nodes of a given secret-sharing
 * contract.
 */
public final class SecretSharingClient<SecretSharesT extends SecretShares> {

  private static final Logger logger = LoggerFactory.getLogger(SecretSharingClient.class);

  /** Constant is set high to cover large numbers of nodes. */
  private static final long GAS_COST_REGISTER_SHARING = 50_000L;

  /** Constant is set high to cover large numbers of nodes. */
  private static final long GAS_COST_REQUEST_DOWNLOAD = 50_000L;

  /**
   * Method for sending transactions. Strategy pattern for testing the {@link SecretSharingClient}.
   */
  public interface TransactionSender {
    /**
     * Method for sending transactions.
     *
     * @param contractAddress Address to send transaction to.
     * @param payload Payload for transaction.
     * @param gasCost Gas cost of transaction.
     */
    void sendAndWaitForInclusion(BlockchainAddress contractAddress, byte[] payload, long gasCost);
  }

  /**
   * Method for sending HTTP requests. Strategy pattern for testing the {@link SecretSharingClient}.
   */
  public interface EndpointHttpClient {
    /**
     * Download a secret share from an execution engine by sending a get request.
     *
     * @param signature the signature to put in the authorization header
     * @param fullUrl Full url of the download.
     * @param timestamp The time when signed
     * @return downloaded secret share
     */
    byte[] downloadShare(Signature signature, String fullUrl, long timestamp);

    /**
     * Upload a secret share to an execution engine by sending a put request.
     *
     * @param signature the signature to put in the authorization header
     * @param fullUrl Full url of the download.
     * @param timestamp The time when signed
     * @param secretShare Secret share data to upload.
     * @return status code.
     */
    int uploadShare(Signature signature, String fullUrl, long timestamp, byte[] secretShare);
  }

  private final TransactionSender transactionSender;
  private final BlockchainAddress offChainSecretSharingContractAddress;
  private final KeyPair senderKey;
  private final OffChainSecretSharing offChainSecretSharingContract;
  private final SecretShares.Factory<SecretSharesT> secretSharesFactory;
  private final EndpointHttpClient endpointHttpClient;
  private final Supplier<Long> timestampSupplier;

  /**
   * Create a new {@link SecretSharingClient}.
   *
   * @param transactionSender An {@link TransactionSender} for interacting with the blockchain.
   * @param offChainSecretSharingContractAddress Address of the secret-sharing smart contract. Not
   *     nullable.
   * @param senderKey {@link KeyPair} to sign transactions using. Not nullable.
   * @param secretSharesFactory Factory for {@link SecretShares}. Used to work with shares.
   * @param offChainSecretSharingContract Client used to access the state of the smart contract.
   * @param endpointHttpClient {@link EndpointHttpClient} used to interact with execution engine
   *     nodes.
   * @param timestampSupplier the function providing timestamps
   */
  private SecretSharingClient(
      final TransactionSender transactionSender,
      final BlockchainAddress offChainSecretSharingContractAddress,
      final KeyPair senderKey,
      final SecretShares.Factory<SecretSharesT> secretSharesFactory,
      final OffChainSecretSharing offChainSecretSharingContract,
      final EndpointHttpClient endpointHttpClient,
      final Supplier<Long> timestampSupplier) {
    this.transactionSender = Objects.requireNonNull(transactionSender);
    this.secretSharesFactory = Objects.requireNonNull(secretSharesFactory);
    this.offChainSecretSharingContractAddress =
        Objects.requireNonNull(offChainSecretSharingContractAddress);
    this.senderKey = Objects.requireNonNull(senderKey);
    this.offChainSecretSharingContract = Objects.requireNonNull(offChainSecretSharingContract);
    this.endpointHttpClient = Objects.requireNonNull(endpointHttpClient);
    this.timestampSupplier = timestampSupplier;
  }

  /**
   * Create a new {@link SecretSharingClient}.
   *
   * @param readerUrl URL of the Partisia Blockchain reader node. Not nullable.
   * @param offChainSecretSharingContractAddress Address of the secret-sharing smart contract. Not
   *     nullable.
   * @param senderKey {@link KeyPair} to sign transactions using. Not nullable.
   */
  public static <SecretSharesT extends SecretShares> SecretSharingClient<SecretSharesT> create(
      final String readerUrl,
      final BlockchainAddress offChainSecretSharingContractAddress,
      final KeyPair senderKey,
      final SecretShares.Factory<SecretSharesT> secretSharesFactory) {

    final BlockchainTransactionClient transactionClient =
        BlockchainTransactionClient.create(readerUrl, new SenderAuthenticationKeyPair(senderKey));

    final TransactionSender transactionSender =
        (BlockchainAddress contractAddress, byte[] payload, long gasCost) -> {
          SentTransaction sentTransaction =
              ExceptionConverter.call(
                  () ->
                      transactionClient.signAndSend(
                          Transaction.create(contractAddress, payload), gasCost));
          transactionClient.waitForSpawnedEvents(sentTransaction);
        };

    return new SecretSharingClient<>(
        transactionSender,
        offChainSecretSharingContractAddress,
        senderKey,
        secretSharesFactory,
        new OffChainSecretSharing(
            BlockchainStateClientImpl.create(readerUrl), offChainSecretSharingContractAddress),
        new JakartaClient(),
        System::currentTimeMillis);
  }

  /**
   * Create new {@link SecretSharingClient} for testing use cases.
   *
   * @param transactionSender An {@link TransactionSender} for interacting with the blockchain.
   * @param offChainSecretSharingContractAddress Address of the secret-sharing smart contract. Not
   *     nullable.
   * @param senderKey {@link KeyPair} to sign transactions using. Not nullable.
   * @param secretSharesFactory Factory for {@link SecretShares}. Used to work with shares.
   * @param offChainSecretSharingContract Client used to access the state of the smart contract.
   * @param endpointHttpClient {@link EndpointHttpClient} used to interact with execution engine
   *     nodes.
   * @param timestampSupplier the function providing timestamps
   */
  public static <SecretSharesT extends SecretShares>
      SecretSharingClient<SecretSharesT> forTestBlockchain(
          final TransactionSender transactionSender,
          final BlockchainAddress offChainSecretSharingContractAddress,
          final KeyPair senderKey,
          final SecretShares.Factory<SecretSharesT> secretSharesFactory,
          final OffChainSecretSharing offChainSecretSharingContract,
          final EndpointHttpClient endpointHttpClient,
          final Supplier<Long> timestampSupplier) {
    return new SecretSharingClient<>(
        transactionSender,
        offChainSecretSharingContractAddress,
        senderKey,
        secretSharesFactory,
        offChainSecretSharingContract,
        endpointHttpClient,
        timestampSupplier);
  }

  /**
   * Register and upload the given plain-text value as secret-shares to the contract's assigned
   * nodes.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @param sharingPlainText Plain text of the secret-sharing. Not nullable.
   * @throws RuntimeException if the sharing id is already in use.
   */
  public void registerAndUploadSharing(BigInteger sharingId, byte[] sharingPlainText) {
    logger.info("Creating shares for sharing {}", sharingId);
    final SecretSharesT shares = createNoncedSharings(sharingPlainText);
    registerSharing(sharingId, shares);
    uploadShares(sharingId, shares);
  }

  /**
   * Create nonce-prefixed secret sharings.
   *
   * @param sharingPlainText The plain text of the secret.
   * @return Secret shares.
   */
  private SecretSharesT createNoncedSharings(byte[] sharingPlainText) {
    final List<OffChainSecretSharing.NodeConfig> nodes = getEngines();
    final SecretSharesT shares =
        secretSharesFactory.fromPlainText(nodes.size(), prefixWithRandomNonce(sharingPlainText));
    return shares;
  }

  private static final SecureRandom RANDOM_GENERATOR = new SecureRandom();

  /**
   * Download the specified sharing and reconstruct the plain-text value from the returned
   * secret-shares.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @return Reconstructed plain-text. Not nullable.
   * @throws RuntimeException if the user doesn't have permission to access the given sharing.
   */
  public byte[] downloadAndReconstruct(BigInteger sharingId) {
    final List<OffChainSecretSharing.NodeConfig> nodes = getEngines();
    logger.info("Request access to secret sharing with id {}", sharingId);
    transactionSender.sendAndWaitForInclusion(
        offChainSecretSharingContractAddress,
        OffChainSecretSharing.requestDownload(sharingId),
        GAS_COST_REQUEST_DOWNLOAD);

    logger.info("Downloading share with id {} from {} engines", sharingId, nodes.size());
    long timestamp = timestampSupplier.get();

    final List<byte[]> shareBytes = new ArrayList<>();
    for (OffChainSecretSharing.NodeConfig node : nodes) {
      final Signature signature =
          createSignatureForOffChainHttpRequest(
              senderKey,
              node.address(),
              offChainSecretSharingContractAddress,
              "GET",
              sharingId,
              timestamp,
              new byte[] {});
      final byte[] receivedShare =
          endpointHttpClient.downloadShare(
              signature, buildUrlForSharing(node, sharingId), timestamp);
      shareBytes.add(receivedShare);
      logger.info("Received share {} from engine: {}", receivedShare, node.address());
    }

    logger.info("All shares received");
    final SecretSharesT shares = secretSharesFactory.fromSharesBytes(shareBytes);
    assertEnginesReturnedCorrectShares(sharingId, shares);

    logger.info("Reconstructing secret");
    return removeNoncePrefix(shares.reconstructPlainText());
  }

  /**
   * Validates that all engines have returned the correct shares. If the haven't it might indicate a
   * malicious party.
   *
   * @param sharingId Sharing identifer. Not nullable.
   * @param shares Shares to validate. Not nullable.
   * @throws RuntimeException if any of the shares doesn't match the expected commitments.
   */
  private void assertEnginesReturnedCorrectShares(BigInteger sharingId, SecretSharesT shares) {
    final List<Hash> expectedCommitments = getExpectedCommitments(sharingId);
    if (!expectedCommitments.equals(shares.commitments())) {
      throw new RuntimeException(
          "Engines did not return the correct secret-shares.\n"
              + "On-chain commitments: %s\nHashes of shares: %s"
                  .formatted(expectedCommitments, shares.commitments()));
    }
  }

  /**
   * Register the sharing with the given identifier.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @param shares Shares to register.
   * @throws RuntimeException if the sharing id is already in use.
   */
  public void registerSharing(BigInteger sharingId, final SecretSharesT shares) {
    logger.info("Registering a new sharing with id {} to the contract", sharingId);
    transactionSender.sendAndWaitForInclusion(
        offChainSecretSharingContractAddress,
        OffChainSecretSharing.registerSharing(sharingId, shares.commitments()),
        GAS_COST_REGISTER_SHARING);
  }

  /**
   * Upload the given plain-text value as secret-shares to the contract's assigned nodes.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @param shares Shares to upload.
   * @throws RuntimeException if the user doesn't have permission to access the given sharing.
   */
  public void uploadShares(BigInteger sharingId, final SecretSharesT shares) {
    final List<OffChainSecretSharing.NodeConfig> nodes = getEngines();
    logger.info("Secret sharing and uploading to id {} for {} engines", sharingId, nodes.size());

    long timestamp = timestampSupplier.get();

    for (int i = 0; i < nodes.size(); i++) {
      final OffChainSecretSharing.NodeConfig node = nodes.get(i);
      final byte[] shareBytes = shares.getShareBytes(i);
      final Signature signature =
          createSignatureForOffChainHttpRequest(
              senderKey,
              node.address(),
              offChainSecretSharingContractAddress,
              "PUT",
              sharingId,
              timestamp,
              shareBytes);
      logger.info("Uploading share {} with id {} to engine", shareBytes, sharingId);
      int responseStatusCode;
      for (int uploadAttempt = 0; uploadAttempt < 10; uploadAttempt++) {
        responseStatusCode =
            endpointHttpClient.uploadShare(
                signature, buildUrlForSharing(node, sharingId), timestamp, shareBytes);
        if (responseStatusCode == 201) {
          break;
        }
        ExceptionConverter.run(() -> Thread.sleep(100));
      }
    }
  }

  /**
   * Get the state of the on-chain secret-sharing contract.
   *
   * @return State of the smart contract. Not nullable.
   */
  public OffChainSecretSharing.ContractState getState() {
    return offChainSecretSharingContract.getState();
  }

  /**
   * Get the list of the node engines assigned to the contract.
   *
   * @return List of assigned node engines. Not nullable.
   */
  public List<OffChainSecretSharing.NodeConfig> getEngines() {
    return getState().nodes();
  }

  /**
   * Get the sharings that have been registered on-chain. Can be used to validate whether the nodes
   * returned the correct shares.
   *
   * @param secretSharingId Identifier of the secret-sharing to get commitments for. Not nullable.
   * @return The share commitments. Not nullable.
   */
  public List<Hash> getExpectedCommitments(BigInteger secretSharingId) {
    return getState().secretSharings().get(secretSharingId).shareCommitments();
  }

  /**
   * Create {@link Signature} for authorization of the http requests against the off chain secret
   * share code.
   *
   * @param senderKey Key to sign signature with.
   * @param engineAddress Address of the node to send the request to.
   * @param contractAddress Address of the contract to send the request to.
   * @param method http method (GET or PUT). Not nullable
   * @param sharingId Identifier of the secret-sharing. Not nullable.
   * @param timestamp The time when signed
   * @param data the secret to send. Not nullable
   * @return the created {@link Signature}. Not nullable.
   */
  public static Signature createSignatureForOffChainHttpRequest(
      final KeyPair senderKey,
      final BlockchainAddress engineAddress,
      final BlockchainAddress contractAddress,
      final String method,
      final BigInteger sharingId,
      final long timestamp,
      final byte[] data) {
    final Hash hash =
        createHashForOffChainHttpRequest(
            engineAddress, contractAddress, method, contractUri(sharingId), timestamp, data);
    return senderKey.sign(hash);
  }

  /**
   * Create {@link Hash} to sign for authorization of the http requests against the off chain secret
   * share code.
   *
   * @param engineAddress Address of the node to send the request to.
   * @param contractAddress Address of the contract to send the request to.
   * @param method http method (GET or PUT). Not nullable
   * @param uri URI of the target endpoint. Not nullable.
   * @param timestamp The time when signed
   * @param data request body. Not nullable
   * @return the created {@link Hash}. Not nullable.
   * @see #createSignatureForOffChainHttpRequest
   */
  public static Hash createHashForOffChainHttpRequest(
      BlockchainAddress engineAddress,
      BlockchainAddress contractAddress,
      String method,
      String uri,
      long timestamp,
      byte[] data) {
    return Hash.create(
        stream -> {
          engineAddress.write(stream);
          contractAddress.write(stream);
          stream.writeString(method);
          stream.writeString(uri);
          stream.writeLong(timestamp);
          stream.writeDynamicBytes(data);
        });
  }

  private String buildUrlForSharing(OffChainSecretSharing.NodeConfig node, BigInteger sharingId) {
    return buildUrlForSharing(offChainSecretSharingContractAddress, node, sharingId);
  }

  private static String buildUrlForSharing(
      BlockchainAddress offChainSecretSharingContractAddress,
      OffChainSecretSharing.NodeConfig node,
      BigInteger sharingId) {
    return node.endpoint()
        + "/offchain/"
        + offChainSecretSharingContractAddress.writeAsString()
        + contractUri(sharingId);
  }

  /**
   * The smart contract's internal URI for a secret-sharing.
   *
   * @param sharingId Identifier of the secret sharing. Not nullable.
   * @return URI to access or modify the secret-sharing. Not nullable.
   */
  public static String contractUri(BigInteger sharingId) {
    return String.format("/shares/%s", sharingId);
  }

  private static final class JakartaClient implements EndpointHttpClient {
    private static final Client CLIENT = getClient();

    @Override
    public byte[] downloadShare(Signature signature, String fullUrl, long timestamp) {
      return CLIENT
          .target(fullUrl)
          .request()
          .header("Authorization", authorizationHeaderValue(signature, timestamp))
          .buildGet()
          .invoke(byte[].class);
    }

    @Override
    public int uploadShare(
        Signature signature, String fullUrl, long timestamp, byte[] secretShare) {
      return CLIENT
          .target(fullUrl)
          .request()
          .header("Authorization", authorizationHeaderValue(signature, timestamp))
          .buildPut(Entity.entity(secretShare, MediaType.APPLICATION_OCTET_STREAM_TYPE))
          .invoke()
          .getStatus();
    }

    private static Client getClient() {
      return ClientBuilder.newBuilder()
          .withConfig(new ResourceConfig(JacksonFeature.class, ObjectMapperProvider.class))
          .connectTimeout(5, TimeUnit.SECONDS)
          .readTimeout(5, TimeUnit.SECONDS)
          .build();
    }
  }

  /** Length of the randomly generated nonce. */
  private static final int NONCE_LENGTH = 32;

  /**
   * Create a share with with a 32-byte nonce prefix and the real data.
   *
   * <p>Inverse of {@link #removeNoncePrefix}.
   *
   * @param data Data to prefix with nonce.
   * @return Prefixed data.
   */
  public static byte[] prefixWithRandomNonce(byte[] data) {
    final byte[] nonce = new byte[NONCE_LENGTH];
    RANDOM_GENERATOR.nextBytes(nonce);
    return SafeDataOutputStream.serialize(
        s -> {
          s.write(nonce);
          s.write(data);
        });
  }

  /**
   * Remove nonce prefix.
   *
   * <p>Inverse of {@link #prefixWithRandomNonce}.
   *
   * @param prefixedData Nonce-prefixed data.
   * @return Data without the nonce-prefix
   */
  public static byte[] removeNoncePrefix(byte[] prefixedData) {
    return Arrays.copyOfRange(prefixedData, NONCE_LENGTH, prefixedData.length);
  }

  /**
   * Create Authorization header value.
   *
   * @param signature the signature of the request
   * @param timestamp the time of the request
   * @return the Authorization header value.
   */
  public static String authorizationHeaderValue(Signature signature, long timestamp) {
    return "secp256k1 " + signature.writeAsString() + " " + timestamp;
  }
}
