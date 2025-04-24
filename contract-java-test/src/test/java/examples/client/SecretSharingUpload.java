package examples.client;

import static java.nio.charset.StandardCharsets.UTF_8;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.crypto.KeyPair;
import java.math.BigInteger;

/**
 * Minimal DEMO program for uploading secret-sharings to secret-sharing contract.
 *
 * <p>Uses a predefined secret-key, and blockchain reader node.
 *
 * <p>Argument format: {@code <CONTRACT ADDRESS> <ID: NUM> <SECRET: STR>}
 */
public final class SecretSharingUpload {

  /** The sender's key. */
  private static final KeyPair SENDER_KEY = new KeyPair(new BigInteger("aa", 16));

  /** Endpoint of the blockchain reader node. */
  private static final String READER_NODE_ENDPOINT = "https://node1.testnet.partisiablockchain.com";

  /**
   * Main method.
   *
   * @param args CLI arguments. Not nullable.
   */
  public static void main(String[] args) {
    // Load arguments
    final BlockchainAddress contractAddress = BlockchainAddress.fromString(args[0]);
    final BigInteger shareId = new BigInteger(args[1]);
    final String secretPlainText = args[2];

    // Upload secret shares
    secretSharingClient(contractAddress)
        .registerAndUploadSharing(shareId, secretPlainText.getBytes(UTF_8));
  }

  /**
   * Create new {@link SecretSharingClient} for the specific contract.
   *
   * @param contractAddress Address of the contract to interact with. Not nullable.
   * @return Client for a secret-sharing smart-contract. Not nullable.
   */
  static SecretSharingClient<XorSecretShares> secretSharingClient(
      BlockchainAddress contractAddress) {
    return new SecretSharingClient<XorSecretShares>(
        READER_NODE_ENDPOINT, contractAddress, SENDER_KEY, XorSecretShares.FACTORY);
  }
}
