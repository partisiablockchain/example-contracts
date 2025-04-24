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
import jakarta.ws.rs.core.Response;
import java.math.BigInteger;
import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.TimeUnit;
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

  private static final Client CLIENT = getClient();

  /** Constant is set high to cover large numbers of nodes. */
  private static final long GAS_COST_REGISTER_SHARING = 50_000L;

  private final String readerUrl;
  private final BlockchainAddress offChainSecretSharingContractAddress;
  private final KeyPair senderKey;
  private final OffChainSecretSharing offChainSecretSharingContract;
  private final SecretShares.Factory<SecretSharesT> secretSharesFactory;

  /**
   * Create a new {@link SecretSharingClient}.
   *
   * @param readerUrl URL of the Partisia Blockchain reader node. Not nullable.
   * @param offChainSecretSharingContractAddress Address of the secret-sharing smart contract. Not
   *     nullable.
   * @param senderKey {@link KeyPair} to sign transactions using. Not nullable.
   */
  public SecretSharingClient(
      final String readerUrl,
      final BlockchainAddress offChainSecretSharingContractAddress,
      final KeyPair senderKey,
      final SecretShares.Factory<SecretSharesT> secretSharesFactory) {
    this.readerUrl = Objects.requireNonNull(readerUrl);
    this.secretSharesFactory = Objects.requireNonNull(secretSharesFactory);
    this.offChainSecretSharingContractAddress =
        Objects.requireNonNull(offChainSecretSharingContractAddress);
    this.senderKey = senderKey;
    this.offChainSecretSharingContract =
        new OffChainSecretSharing(
            BlockchainStateClientImpl.create(readerUrl), offChainSecretSharingContractAddress);
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
    final SecretSharesT shares = createNoncedSharings(sharingId, sharingPlainText);
    registerSharing(sharingId, shares);
    uploadShares(sharingId, shares);
  }

  private SecretSharesT createNoncedSharings(BigInteger sharingId, byte[] sharingPlainText) {
    final List<OffChainSecretSharing.NodeConfig> nodes = getEngines();
    logger.info("Creating shares for sharing {}, using {} engines", sharingId, nodes.size());
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
    logger.info("Downloading share with id {} from {} engines", sharingId, nodes.size());

    final List<byte[]> shareBytes = new ArrayList<>();
    for (OffChainSecretSharing.NodeConfig node : nodes) {
      final Signature signature =
          createSignatureForOffChainHttpRequest(
              senderKey,
              node.address(),
              offChainSecretSharingContractAddress,
              "GET",
              sharingId,
              new byte[] {});
      final byte[] receivedShare = downloadShare(signature, buildUrlForSharing(node, sharingId));
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

    byte[] registerSharingPayload =
        OffChainSecretSharing.registerSharing(sharingId, shares.commitments());

    final BlockchainTransactionClient transactionClient =
        BlockchainTransactionClient.create(readerUrl, new SenderAuthenticationKeyPair(senderKey));

    SentTransaction sentTransaction =
        ExceptionConverter.call(
            () ->
                transactionClient.signAndSend(
                    Transaction.create(
                        offChainSecretSharingContractAddress, registerSharingPayload),
                    GAS_COST_REGISTER_SHARING));
    transactionClient.waitForSpawnedEvents(sentTransaction);
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
              shareBytes);
      logger.info("Uploading share {} with id {} to engine", shareBytes, sharingId);
      uploadShare(signature, buildUrlForSharing(node, sharingId), shareBytes);
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
   * @param data the secret to send. Not nullable
   * @return the created {@link Signature}. Not nullable.
   */
  public static Signature createSignatureForOffChainHttpRequest(
      final KeyPair senderKey,
      final BlockchainAddress engineAddress,
      final BlockchainAddress contractAddress,
      final String method,
      final BigInteger sharingId,
      final byte[] data) {
    final Hash hash =
        createHashForOffChainHttpRequest(
            engineAddress, contractAddress, method, contractUri(sharingId), data);
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
   * @param data request body. Not nullable
   * @return the created {@link Hash}. Not nullable.
   * @see #createSignatureForOffChainHttpRequest
   */
  public static Hash createHashForOffChainHttpRequest(
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

  private String buildUrlForSharing(OffChainSecretSharing.NodeConfig node, BigInteger sharingId) {
    return buildUrlForSharing(offChainSecretSharingContractAddress, node, sharingId);
  }

  private static String buildUrlForSharing(
      BlockchainAddress offChainSecretSharingContractAddress,
      OffChainSecretSharing.NodeConfig node,
      BigInteger sharingId) {
    return node.endpoint()
        + "/executioncontainer/"
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

  /**
   * Download a secret share from an execution engine by sending a get request.
   *
   * @param signature the signature to put in the authorization header
   * @return downloaded secret share
   */
  private static byte[] downloadShare(Signature signature, String fullUrl) {
    return CLIENT
        .target(fullUrl)
        .request()
        .header("Authorization", "secp256k1 " + signature.writeAsString())
        .get(byte[].class);
  }

  private static Client getClient() {
    return ClientBuilder.newBuilder()
        .withConfig(new ResourceConfig(JacksonFeature.class, ObjectMapperProvider.class))
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(5, TimeUnit.SECONDS)
        .build();
  }

  /**
   * Upload a secret share to an execution engine by sending a put request.
   *
   * @param signature the signature to put in the authorization header
   */
  private static void uploadShare(Signature signature, String fullUrl, byte[] secretShare) {
    Response response;
    do {
      response =
          CLIENT
              .target(fullUrl)
              .request()
              .header("Authorization", "secp256k1 " + signature.writeAsString())
              .put(Entity.entity(secretShare, MediaType.APPLICATION_OCTET_STREAM_TYPE));
    } while (response.getStatus() != 201);
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
}
