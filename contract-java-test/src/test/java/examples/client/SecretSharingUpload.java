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
 * <p>Argument format: {@code <SHARING MODE> <CONTRACT ADDRESS> <ID: NUM> <SECRET: STR>}
 *
 * <p>Where {@code <SHARING MODE>} is either {@code "xor"} or {@code "shamir"}
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
    final String sharingMode = args[0];
    final SecretShares.Factory<?> factory = getFactory(sharingMode);
    final BlockchainAddress contractAddress = BlockchainAddress.fromString(args[1]);
    final BigInteger shareId = new BigInteger(args[2]);
    final String secretPlainText = args[3];

    // Upload secret shares
    secretSharingClient(contractAddress, factory)
        .registerAndUploadSharing(shareId, secretPlainText.getBytes(UTF_8));
  }

  /**
   * Get the corresponding factory according to the supplied sharing mode.
   *
   * @param sharingMode "xor" for xor secret sharing or "shamir" for shamir secret sharing
   * @return the secret sharing factory
   * @throws RuntimeException if the supplied sharing mode is neither "xor" nor "shamir"
   */
  static SecretShares.Factory<?> getFactory(String sharingMode) {
    if (sharingMode.equals("xor")) {
      return XorSecretShares.FACTORY;
    } else if (sharingMode.equals("shamir")) {
      return ShamirSecretShares.FACTORY;
    } else {
      throw new RuntimeException("Invalid secret sharing mode. Valid modes are [xor, shamir].");
    }
  }

  /**
   * Create new {@link SecretSharingClient} for the specific contract.
   *
   * @param contractAddress Address of the contract to interact with. Not nullable.
   * @param factory factory for creating secret shares
   * @return Client for a secret-sharing smart-contract. Not nullable.
   */
  static <T extends SecretShares> SecretSharingClient<T> secretSharingClient(
      BlockchainAddress contractAddress, SecretShares.Factory<T> factory) {
    return SecretSharingClient.create(READER_NODE_ENDPOINT, contractAddress, SENDER_KEY, factory);
  }
}
