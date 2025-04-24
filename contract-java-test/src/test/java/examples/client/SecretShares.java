package examples.client;

import com.partisiablockchain.crypto.Hash;
import java.util.List;
import java.util.stream.IntStream;

/**
 * All shares of a specific secure secret-sharing scheme.
 *
 * <p>Constructed either from a plain-text or from all shares of the plain-text. Can be converted
 * into the individual shares, or into the plain-text.
 *
 * @see <a href="https://en.wikipedia.org/wiki/Secret_sharing">Secret sharing, Wikipedia</a>
 */
public interface SecretShares {

  /** Get number of shares that this {@link SecretShares} have been split over. */
  int numShares();

  /** Get the secret-share for the given node index. */
  byte[] getShareBytes(int nodeIndex);

  /** Reconstruct plaintext from all the shares of the {@link SecretShares} object. */
  byte[] reconstructPlainText();

  /**
   * Get commitments for all shares.
   *
   * @return all commitments.
   */
  default List<Hash> commitments() {
    return IntStream.range(0, numShares())
        .mapToObj(this::getShareBytes)
        .map(SecretShares::createShareCommitment)
        .toList();
  }

  /**
   * Create an individual share commitment.
   *
   * @param share Share to create commitment for.
   * @return Commitment as a {@link Hash}.
   */
  static Hash createShareCommitment(byte[] share) {
    return Hash.create(stream -> stream.write(share));
  }

  /**
   * Factory object for create {@link SecretShares}.
   *
   * @param <SecretSharesT> Type of {@link SecretShares} create by the factory object.
   */
  interface Factory<SecretSharesT extends SecretShares> {
    /**
     * Create a new {@link SecretShares} from the given plaintext for the given amount of recipient
     * nodes.
     *
     * <p>The factory is allowed to fail if it doesn't support the given amount of recipient nodes.
     *
     * @param numNodes Number of nodes to spread secret-shares across.
     * @param plainText The plain text to be secret-shared. Not nullable.
     * @return Newly created {@link SecretSharesT}. Not nullable.
     */
    SecretSharesT fromPlainText(int numNodes, byte[] plainText);

    /**
     * Create a new {@link SecretShares} from the given shares.
     *
     * @param shares Shares to create from.
     * @return Newly created {@link SecretSharesT}. Not nullable.
     */
    SecretSharesT fromSharesBytes(List<byte[]> shares);
  }
}
