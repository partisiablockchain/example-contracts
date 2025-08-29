package examples.client;

import com.partisiablockchain.zk.real.protocol.field.Lagrange;
import com.partisiablockchain.zk.real.protocol.field.Polynomial;
import com.secata.stream.SafeDataOutputStream;
import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

/**
 * Secure secret-sharing scheme based on shamir-secret sharing.
 *
 * @see <a href="https://en.wikipedia.org/wiki/Shamir%27s_secret_sharing">Shamir's secret sharing,
 *     Wikipedia</a>
 */
public final class ShamirSecretShares implements SecretShares {

  /** Factory for creating instances of {@link ShamirSecretShares}. */
  public static Factory<ShamirSecretShares> FACTORY = new ShamirFactory(new ShamirConfig(1, 4, 2));

  private static final SecureRandom RANDOM_GENERATOR = new SecureRandom();

  private final ShamirConfig shamirConfig;
  private final List<Share> shares;

  private ShamirSecretShares(ShamirConfig shamirConfig, List<Share> shares) {
    this.shamirConfig = shamirConfig;
    this.shares = Objects.requireNonNull(shares);
    if (shares.size() != shamirConfig.numNodes) {
      throw new IllegalArgumentException(
          "ShamirSecretShares requires at %d shares, but %d were specified"
              .formatted(shamirConfig.numNodes, shares.size()));
    }
    long numReceivedShares = shares.stream().filter(Objects::nonNull).count();
    if (numReceivedShares < shamirConfig.numToReconstruct) {
      throw new IllegalArgumentException(
          "Must have received at least %d shares to reconstruct. Received %d."
              .formatted(shamirConfig.numToReconstruct, numReceivedShares));
    }
  }

  static ShamirSecretShares fromPlainText(ShamirConfig shamirConfig, byte[] plainText) {
    return new ShamirSecretShares(shamirConfig, createSecretShares(shamirConfig, plainText));
  }

  static ShamirSecretShares fromSharesBytes(ShamirConfig shamirConfig, List<byte[]> shareBytes) {
    return new ShamirSecretShares(
        shamirConfig,
        shareBytes.stream()
            .map(
                bytes -> {
                  if (bytes == null) {
                    return null;
                  }
                  List<F256> shares = new ArrayList<>();
                  for (byte byteValue : bytes) {
                    shares.add(F256.create(byteValue));
                  }
                  return new Share(shares);
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
   * @param shamirConfig shamir config for determining the degree of the polynomial
   * @param secret the secret to be embedded in the constant term
   * @return a random polynomial generated with the secret and a random number as coefficients.
   */
  private static Polynomial<F256> generatePolynomial(ShamirConfig shamirConfig, F256 secret) {
    byte[] randomByte = new byte[shamirConfig.numMalicious()];
    RANDOM_GENERATOR.nextBytes(randomByte);
    final List<F256> coefficients = new ArrayList<>(List.of(secret));
    for (int i = 0; i < shamirConfig.numMalicious(); i++) {
      final F256 random = F256.create(randomByte[i]);
      coefficients.add(random);
    }
    return new Polynomial<>(coefficients, F256.ZERO);
  }

  private static List<Share> createSecretShares(ShamirConfig shamirConfig, byte[] variableData) {
    List<F256> alphas = F256.alphas(shamirConfig.numNodes);

    List<List<F256>> sharedElements = new ArrayList<>(shamirConfig.numNodes);
    for (int i = 0; i < shamirConfig.numNodes; i++) {
      sharedElements.add(i, new ArrayList<>());
    }

    for (byte secretByte : variableData) {
      F256 secret = F256.create(secretByte);

      Polynomial<F256> polynomial = generatePolynomial(shamirConfig, secret);
      for (int j = 0; j < shamirConfig.numNodes; j++) {
        F256 share = polynomial.evaluate(alphas.get(j));
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
    return reconstructPlainTextBytes();
  }

  /**
   * Reconstruct each byte and write them to the output stream.
   *
   * @return the reconstructed plain text
   */
  private byte[] reconstructPlainTextBytes() {
    List<F256> alphas = F256.alphas(shamirConfig.numNodes());
    List<F256> definedAlphas = new ArrayList<>();
    List<Share> definedShares = new ArrayList<>();
    for (int i = 0; i < shamirConfig.numNodes(); i++) {
      if (shares.get(i) != null) {
        definedAlphas.add(alphas.get(i));
        definedShares.add(shares.get(i));
      }
    }

    int shareByteLength = definedShares.get(0).share().size();
    byte[] result = new byte[shareByteLength];
    for (int i = 0; i < shareByteLength; i++) {
      int finalI = i;
      List<F256> elementShares =
          definedShares.stream().map(share -> share.share.get(finalI)).toList();
      result[i] = (byte) reconstructPlainTextByte(definedAlphas, elementShares);
    }
    return result;
  }

  /**
   * Reconstructs one byte of the secret variable data which these binary secret shares constitute.
   * A polynomial is interpolated from the shares of the byte, and the constant term of this
   * polynomial is returned as the reconstructed byte.
   *
   * @param definedAlphas alphas of the present shares
   * @param elementShares the element shares for the given element
   * @return the reconstructed byte
   */
  private int reconstructPlainTextByte(List<F256> definedAlphas, List<F256> elementShares) {
    Polynomial<F256> polynomial =
        Lagrange.interpolateIfPossible(
            definedAlphas, elementShares, shamirConfig.numMalicious(), F256.ZERO, F256.ONE);
    if (polynomial == null) {
      throw new RuntimeException("Unable to reconstruct secret");
    }
    return polynomial.getConstantTerm().intValue();
  }

  private record Share(List<F256> share) {

    /**
     * Serializes each bit element of the share.
     *
     * @return the serialized share
     */
    private byte[] serialize() {
      return SafeDataOutputStream.serialize(
          stream -> {
            for (F256 byteElement : share) {
              stream.write(byteElement.serialize());
            }
          });
    }
  }

  /**
   * Configuration for creating and reconstructing shamir secret shares.
   *
   * <p>The {@code numToReconstruct} value must be at least {@code numMalicious + 1} to be able to
   * reconstruct.
   *
   * <p>If {@code numToReconstruct} is less than {@code 2 * numMalicious + 1} then there must be
   * some other way to guarantee that the shares are correct, e.g. through commitments on-chain.
   *
   * @param numMalicious maximum number of malicious nodes. Depends on the threat model.
   * @param numNodes total number of nodes to receive a share.
   * @param numToReconstruct minimum number of shares needed to reconstruct a secret.
   */
  public record ShamirConfig(int numMalicious, int numNodes, int numToReconstruct) {}

  /** Factory for creating elements of {@link ShamirSecretShares}. */
  public static final class ShamirFactory implements SecretShares.Factory<ShamirSecretShares> {

    private final ShamirConfig shamirConfig;

    /**
     * Create the {@link ShamirFactory}.
     *
     * @param shamirConfig configuration for creating and reconstructing shamir secret shares.
     */
    public ShamirFactory(ShamirConfig shamirConfig) {
      this.shamirConfig = shamirConfig;
    }

    @Override
    public ShamirSecretShares fromPlainText(int numNodes, byte[] plainText) {
      if (numNodes != shamirConfig.numNodes()) {
        throw new IllegalArgumentException(
            "This shamir factory expects there to be %d nodes, but there was %d."
                .formatted(shamirConfig.numNodes(), numNodes));
      }
      return ShamirSecretShares.fromPlainText(shamirConfig, plainText);
    }

    @Override
    public ShamirSecretShares fromSharesBytes(List<byte[]> shares) {
      return ShamirSecretShares.fromSharesBytes(shamirConfig, shares);
    }
  }
}
