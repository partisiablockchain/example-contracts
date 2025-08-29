package examples.client;

import static java.nio.charset.StandardCharsets.UTF_8;

import com.partisiablockchain.BlockchainAddress;
import java.math.BigInteger;

/**
 * Minimal DEMO program for download secret-sharings to secret-sharing contract.
 *
 * <p>Uses a predefined secret-key, and blockchain reader node.
 *
 * <p>Argument format: {@code <SHARING MODE> <CONTRACT ADDRESS> <ID: NUM>}
 *
 * <p>Where {@code <SHARING MODE>} is either {@code "xor"} or {@code "shamir"}
 */
public final class SecretSharingDownload {

  /**
   * Main method.
   *
   * @param args CLI arguments. Not nullable.
   */
  public static void main(String[] args) {
    // Load arguments
    final String sharingMode = args[0];
    final SecretShares.Factory<?> factory = SecretSharingUpload.getFactory(sharingMode);
    final BlockchainAddress contractAddress = BlockchainAddress.fromString(args[1]);
    final BigInteger shareId = new BigInteger(args[2]);

    // Download secret sharing
    final byte[] reconstructedSecret =
        SecretSharingUpload.secretSharingClient(contractAddress, factory)
            .downloadAndReconstruct(shareId);

    // Print it
    System.out.println(new String(reconstructedSecret, UTF_8));
  }
}
