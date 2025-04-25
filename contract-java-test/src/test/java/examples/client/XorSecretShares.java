package examples.client;

import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

/**
 * Secure secret-sharing scheme based on XOR.
 *
 * <p>Supported number of shares ({@code N}): {@code 2+}
 *
 * <p>Threshold for reconstruction: {@code N}
 *
 * @see <a href="https://en.wikipedia.org/wiki/Secret_sharing">Secret sharing, Wikipedia</a>
 */
public final class XorSecretShares implements SecretShares {

  /** The least number of shares supported by the secret sharing. */
  public static int MIN_SHARES = 2;

  /** Factory for creating instances of {@link XorSecretShares}. */
  public static Factory<XorSecretShares> FACTORY =
      new Factory<XorSecretShares>() {
        @Override
        public XorSecretShares fromPlainText(int numNodes, byte[] plainText) {
          return XorSecretShares.fromPlainText(numNodes, plainText);
        }

        @Override
        public XorSecretShares fromSharesBytes(List<byte[]> shares) {
          return XorSecretShares.fromSharesBytes(shares);
        }
      };

  private static final SecureRandom RANDOM_GENERATOR = new SecureRandom();

  private final List<byte[]> shares;

  private XorSecretShares(List<byte[]> shares) {
    this.shares = Objects.requireNonNull(shares);
    if (shares.size() < MIN_SHARES) {
      throw new IllegalArgumentException(
          "XorSecretShares requires at least %d shares, but specified only %d"
              .formatted(MIN_SHARES, shares.size()));
    }
  }

  static XorSecretShares fromPlainText(int numNodes, byte[] serializedSecret) {
    return new XorSecretShares(createSecretShares(numNodes, serializedSecret));
  }

  static XorSecretShares fromSharesBytes(List<byte[]> shares) {
    return new XorSecretShares(shares);
  }

  @Override
  public int numShares() {
    return shares.size();
  }

  @Override
  public byte[] getShareBytes(int nodeIndex) {
    return shares.get(nodeIndex);
  }

  @Override
  public byte[] reconstructPlainText() {
    return xorByteArrays(shares.get(0).length, shares);
  }

  private static List<byte[]> createSecretShares(int numNodes, byte[] variableData) {
    final List<byte[]> allShares = new ArrayList<>();

    for (int idx = 0; idx < numNodes - 1; idx++) {
      final byte[] share = new byte[variableData.length];
      RANDOM_GENERATOR.nextBytes(share);
      allShares.add(share);
    }

    final byte[] lastShare = xorByteArrays(variableData.length, allShares);
    xorBytesAndStoreIntoFirst(lastShare, variableData);
    allShares.add(lastShare);
    return List.copyOf(allShares);
  }

  private static byte[] xorByteArrays(final int length, final List<byte[]> manyBytes) {
    final byte[] temporaryResult = new byte[length];
    for (final byte[] bytes : manyBytes) {
      xorBytesAndStoreIntoFirst(temporaryResult, bytes);
    }
    return temporaryResult;
  }

  private static void xorBytesAndStoreIntoFirst(final byte[] bytes1, final byte[] bytes2) {
    for (int idx = 0; idx < bytes2.length; idx++) {
      bytes1[idx] = (byte) (bytes1[idx] ^ bytes2[idx]);
    }
  }
}
