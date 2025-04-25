package examples.client;

import static java.nio.charset.StandardCharsets.UTF_8;

import com.partisiablockchain.BlockchainAddress;
import java.math.BigInteger;

/**
 * Minimal DEMO program for download secret-sharings to secret-sharing contract.
 *
 * <p>Uses a predefined secret-key, and blockchain reader node.
 *
 * <p>Argument format: {@code <CONTRACT ADDRESS> <ID: NUM>}
 */
public final class SecretSharingDownload {

  /**
   * Main method.
   *
   * @param args CLI arguments. Not nullable.
   */
  public static void main(String[] args) {
    // Load arguments
    final BlockchainAddress contractAddress = BlockchainAddress.fromString(args[0]);
    final BigInteger shareId = new BigInteger(args[1]);

    // Download secret sharing
    final byte[] reconstructedSecret =
        SecretSharingUpload.secretSharingClient(contractAddress).downloadAndReconstruct(shareId);

    // Print it
    System.out.println(new String(reconstructedSecret, UTF_8));
  }
}
