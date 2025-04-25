package examples.client;

import com.partisiablockchain.zk.real.protocol.binary.field.BinaryExtensionFieldElement;
import com.partisiablockchain.zk.real.protocol.binary.field.BinaryExtensionFieldElementFactory;
import com.partisiablockchain.zk.real.protocol.field.Lagrange;
import com.partisiablockchain.zk.real.protocol.field.Polynomial;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import com.secata.stream.SafeDataInputStream;
import com.secata.stream.SafeDataOutputStream;
import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;

/**
 * Secure secret-sharing scheme based on shamir-secret sharing.
 *
 * <p>Supported number of shares ({@code N}): {@code 4}
 *
 * @see <a href="https://en.wikipedia.org/wiki/Shamir%27s_secret_sharing">Shamir's secret sharing,
 *     Wikipedia</a>
 */
public final class ShamirSecretShares implements SecretShares {

  /** Number of shares the secret sharing requires. */
  public static int NUM_SHARES = 4;

  /** Factory for creating instances of {@link ShamirSecretShares}. */
  public static Factory<ShamirSecretShares> FACTORY =
      new Factory<ShamirSecretShares>() {
        @Override
        public ShamirSecretShares fromPlainText(int numNodes, byte[] plainText) {
          return ShamirSecretShares.fromPlainText(numNodes, plainText);
        }

        @Override
        public ShamirSecretShares fromSharesBytes(List<byte[]> shares) {
          return ShamirSecretShares.fromSharesBytes(shares);
        }
      };

  private static final SecureRandom RANDOM_GENERATOR = new SecureRandom();

  private final List<Share> shares;

  static final List<BinaryExtensionFieldElement> ALPHAS =
      new BinaryExtensionFieldElementFactory().alphas();

  private ShamirSecretShares(List<Share> shares) {
    this.shares = Objects.requireNonNull(shares);
    if (shares.size() != NUM_SHARES) {
      throw new IllegalArgumentException(
          "ShamirSecretShares requires at %d shares, but %d were specified"
              .formatted(NUM_SHARES, shares.size()));
    }
  }

  static ShamirSecretShares fromPlainText(int numNodes, byte[] plainText) {
    return fromPlainText(numNodes, BitOutput.serializeBits(out -> out.writeBytes(plainText)));
  }

  static ShamirSecretShares fromPlainText(int numNodes, CompactBitArray plainText) {
    return new ShamirSecretShares(createSecretShares(numNodes, plainText));
  }

  static ShamirSecretShares fromSharesBytes(List<byte[]> shares) {
    return new ShamirSecretShares(
        shares.stream()
            .map(SafeDataInputStream::createFromBytes)
            .map(
                stream -> {
                  int numOfElements = 1;
                  List<BinaryExtensionFieldElement> sharesOfBits = new ArrayList<>();
                  for (int i = 0; i < numOfElements; i++) {
                    int bitLength = 32;
                    for (int j = 0; j < bitLength; j++) {
                      BinaryExtensionFieldElement shareOfBit =
                          BinaryExtensionFieldElement.create(stream.readSignedByte());
                      sharesOfBits.add(shareOfBit);
                    }
                  }
                  return new Share(sharesOfBits);
                })
            .toList());
  }

  @Override
  public int numShares() {
    return shares.size();
  }

  @Override
  public byte[] getShareBytes(int nodeIndex) {
    return shares.get(nodeIndex).serialize();
  }

  /**
   * Generates a random polynomial of degree 1:
   *
   * <p><i>f(x)= secret + random*x</i>
   *
   * <p>such that f(0) match the provided secret.
   *
   * @param secret the secret to be embedded in the constant term
   * @return a random polynomial generated with the secret and a random number as coefficients.
   */
  private static Polynomial<BinaryExtensionFieldElement> generatePolynomial(
      BinaryExtensionFieldElement secret) {
    byte[] randomByte = new byte[1];
    RANDOM_GENERATOR.nextBytes(randomByte);
    final BinaryExtensionFieldElement random = BinaryExtensionFieldElement.create(randomByte[0]);
    final List<BinaryExtensionFieldElement> coefficients = List.of(secret, random);
    return new Polynomial<>(coefficients, BinaryExtensionFieldElement.ZERO);
  }

  private static boolean isBitSet(byte[] data, int index) {
    final int byteIndex = index / 8;
    final byte byteValue = data[byteIndex];
    final int bitIndex = index % 8;
    final int bitValue = byteValue & (1 << bitIndex);
    return bitValue != 0;
  }

  private static List<Share> createSecretShares(int numNodes, CompactBitArray variableData) {
    if (numNodes != ALPHAS.size()) {
      throw new IllegalArgumentException(
          "ShamirSecretShares supports precisely %s nodes, but tried to create %s secret-shares"
              .formatted(ALPHAS.size(), numNodes));
    }

    List<List<BinaryExtensionFieldElement>> sharedElements = new ArrayList<>(ALPHAS.size());
    for (int i = 0; i < ALPHAS.size(); i++) {
      sharedElements.add(i, new ArrayList<>());
    }

    for (int i = 0; i < variableData.size(); i++) {
      BinaryExtensionFieldElement secret =
          isBitSet(variableData.data(), i)
              ? BinaryExtensionFieldElement.ONE
              : BinaryExtensionFieldElement.ZERO;

      Polynomial<BinaryExtensionFieldElement> polynomial = generatePolynomial(secret);
      for (int j = 0; j < ALPHAS.size(); j++) {
        BinaryExtensionFieldElement share = polynomial.evaluate(ALPHAS.get(j));
        sharedElements.get(j).add(share);
      }
    }

    return sharedElements.stream().map(Share::new).toList();
  }

  /**
   * Reconstruct the secret variable data from these BinarySecretShares. First the shares of each
   * bit of the secret variable data is grouped. Then, a polynomial is interpolated from the shares
   * of each bit. The constant term of this polynomial is the value of the secret bit. Lastly, the
   * secret bits are collected in a byte array to form the secret variable data.
   *
   * @return the reconstructed binary secret variable data
   */
  @Override
  public byte[] reconstructPlainText() {
    return reconstructPlainTextBits().data();
  }

  private CompactBitArray reconstructPlainTextBits() {
    return BitOutput.serializeBits(this::reconstructPlainTextBits);
  }

  /**
   * Reconstruct each bit and write them to the bit output.
   *
   * @param bitOutput bit output stream to write to
   */
  private void reconstructPlainTextBits(BitOutput bitOutput) {
    for (int i = 0; i < getShareBitLength(); i++) {
      bitOutput.writeBoolean(reconstructPlainTextBit(i));
    }
  }

  /**
   * Reconstructs one bit of the secret variable data which these binary secret shares constitute. A
   * polynomial is interpolated from the shares of the bit, and the constant term of this polynomial
   * is returned as the reconstructed bit.
   *
   * @param i index of the bit to reconstruct
   * @return the reconstructed bit
   */
  private boolean reconstructPlainTextBit(int i) {
    List<BinaryExtensionFieldElement> sharesOfBit =
        shares.stream().map(share -> share.share.get(i)).collect(Collectors.toList());
    Polynomial<BinaryExtensionFieldElement> polynomial = interpolatePolynomial(sharesOfBit);
    return !polynomial.getConstantTerm().isZero();
  }

  /**
   * Interpolates a polynomial from a list of shares using lagrange interpolation.
   *
   * @param shares the shares which acts as y-coordinates of the points on the polynomial
   * @return the interpolated polynomial
   */
  private static Polynomial<BinaryExtensionFieldElement> interpolatePolynomial(
      List<BinaryExtensionFieldElement> shares) {
    return Lagrange.interpolate(
        ALPHAS, shares, 1, BinaryExtensionFieldElement.ZERO, BinaryExtensionFieldElement.ONE);
  }

  /**
   * Get the bitlength of the secret sharing.
   *
   * @return the numver of bits secret shared.
   */
  private int getShareBitLength() {
    return shares.get(0).share.size();
  }

  private record Share(List<BinaryExtensionFieldElement> share) {

    /**
     * Serializes each bit element of the share.
     *
     * @return the serialized share
     */
    public byte[] serialize() {
      return SafeDataOutputStream.serialize(
          stream -> {
            for (BinaryExtensionFieldElement bitElement : share) {
              stream.write(bitElement.serialize());
            }
          });
    }
  }
}
