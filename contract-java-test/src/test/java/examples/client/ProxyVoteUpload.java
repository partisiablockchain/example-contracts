package examples.client;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.Voting;
import com.partisiablockchain.language.abistreams.AbiByteOutput;
import com.partisiablockchain.language.abistreams.AbiOutput;
import com.secata.tools.rest.ObjectMapperProvider;
import jakarta.ws.rs.client.ClientBuilder;
import jakarta.ws.rs.client.Entity;
import jakarta.ws.rs.core.HttpHeaders;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.util.concurrent.TimeUnit;
import org.glassfish.jersey.jackson.JacksonFeature;
import org.glassfish.jersey.server.ResourceConfig;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Script that uploads a {@link Vote} transaction request to an execution engine of a {@link Proxy}
 * contract.
 *
 * <p>Argument format: {@code <true/false> <AUTHORIZATION_TOKEN> <VOTE_CONTRACT_ADDRESS>
 * <PROXY_ENDPOINT>}
 */
public final class ProxyVoteUpload {
  private static final Logger logger = LoggerFactory.getLogger(SecretSharingClient.class);

  /** Gas cost that is high enough to cover all associated costs of proxying the transaction. */
  private static final int GAS_COST = 2_600;

  /**
   * Request from a user to proxy a transaction to a target smart contract.
   *
   * <p>Contains the transaction details and gas cost needed by the off-chain execution engine to
   * forward the request. This is sent in HTTP request bodies to the proxy contract's off-chain
   * component.
   *
   * @param targetSmartContract The target smart contract address to forward the payload to.
   * @param payload The serialized transaction to be forwarded.
   * @param gas The total gas needed for the transaction, in gas units.
   */
  @SuppressWarnings("ArrayRecordComponent")
  public record TransactionRequest(
      BlockchainAddress targetSmartContract, byte[] payload, long gas) {
    /**
     * Serializes the transaction request.
     *
     * @param out Output stream to write serialized bytes to
     */
    public void serialize(AbiOutput out) {
      out.writeAddress(targetSmartContract);
      out.writeI32(payload.length);
      out.writeBytes(payload);
      out.writeI64(gas);
    }
  }

  /**
   * Main method that takes in arguments from the CLI that describe how to construct the transaction
   * request.
   *
   * <p>It takes in four arguments. (1) {@code true/false} that indicates whether the vote should be
   * true or false (2) {@code <AUTHORIZATION_TOKEN>} which is the token that should be used for
   * authorization with the execution engine (3) {@code <VOTE_CONTRACT_ADDRESS>} which is contract
   * address, as a hexadecimal string, of the target voting contract and (4) {@code
   * <PROXY_ENDPOINT>} a full endpoint of an execution engine of the proxy contract to upload to.
   *
   * @param args CLI arguments. Not nullable.
   */
  public static void main(String[] args) {
    // Parse arguments
    if (args.length != 4) {
      logger.error(
          "Incorrect number of arguments. Program expects argument format: <true/false>"
              + " <AUTHORIZATION_TOKEN> <VOTE_CONTRACT_ADDRESS>"
              + " <PROXY_ENDPOINT>");
      return;
    }
    boolean vote = Boolean.parseBoolean(args[0]);
    String authToken = args[1];
    BlockchainAddress votingContractAddress = BlockchainAddress.fromString(args[2]);
    String proxyEndpoint = args[3];

    // Build transaction request bytes
    TransactionRequest transactionRequest =
        new TransactionRequest(votingContractAddress, Voting.vote(vote), GAS_COST);
    byte[] transactionBytes = AbiByteOutput.serializeBigEndian(transactionRequest::serialize);

    // Upload an HTTP request to the execution engine and log its response
    Response response =
        ClientBuilder.newBuilder()
            .withConfig(new ResourceConfig(JacksonFeature.class, ObjectMapperProvider.class))
            .connectTimeout(5, TimeUnit.SECONDS)
            .readTimeout(5, TimeUnit.SECONDS)
            .build()
            .target(proxyEndpoint)
            .request()
            .header(HttpHeaders.AUTHORIZATION, "Bearer " + authToken)
            .buildPost(Entity.entity(transactionBytes, MediaType.APPLICATION_OCTET_STREAM_TYPE))
            .invoke();

    logger.info(
        "Off chain responded with status code {} and data \"{}\"",
        response.getStatus(),
        response.readEntity(String.class));
  }
}
