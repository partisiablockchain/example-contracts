package examples.client;

import static java.nio.charset.StandardCharsets.UTF_8;

import java.util.ArrayList;
import java.util.List;
import java.util.stream.IntStream;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.Test;

/** Test of {@link SecretShares}. */
public final class SecretSharesTest {

  private static final byte[] SECRET_PLAIN_TEXT = "Hello, World!".getBytes(UTF_8);

  /** From plain text generates the correct amount of shares. */
  @Test
  public void fromPlainTextCorrectNumShares() {
    for (int numShares = 2; numShares < 20; numShares++) {
      final XorSecretShares share =
          XorSecretShares.FACTORY.fromPlainText(numShares, SECRET_PLAIN_TEXT);
      Assertions.assertThat(share.numShares()).as("numShares").isEqualTo(numShares);
      Assertions.assertThat(share.commitments()).as("commitments").hasSize(numShares);
    }
  }

  /** Test that plain-text can be reconstructed from {@link XorSecretShares}. */
  @Test
  public void reconstructXorSecretShares() {
    Assertions.assertThat(
            XorSecretShares.FACTORY.fromPlainText(2, SECRET_PLAIN_TEXT).reconstructPlainText())
        .isEqualTo(SECRET_PLAIN_TEXT);
    Assertions.assertThat(
            XorSecretShares.FACTORY.fromPlainText(5, SECRET_PLAIN_TEXT).reconstructPlainText())
        .isEqualTo(SECRET_PLAIN_TEXT);
  }

  /** Test that plain-text can be reconstructed from {@link ShamirSecretShares}. */
  @Test
  public void reconstructShamirSecretShares() {
    ShamirSecretShares shamirSecretShares =
        ShamirSecretShares.FACTORY.fromPlainText(4, SECRET_PLAIN_TEXT);

    List<byte[]> rawShares =
        IntStream.range(0, 4).mapToObj(shamirSecretShares::getShareBytes).toList();
    ShamirSecretShares readShares = ShamirSecretShares.FACTORY.fromSharesBytes(rawShares);

    Assertions.assertThat(readShares.reconstructPlainText()).isEqualTo(SECRET_PLAIN_TEXT);
  }

  /**
   * Test that plain-text can be reconstructed from {@link ShamirSecretShares} with missing shares.
   */
  @Test
  public void reconstructShamirSecretSharesMissingShares() {
    ShamirSecretShares shamirSecretShares =
        ShamirSecretShares.FACTORY.fromPlainText(4, SECRET_PLAIN_TEXT);

    List<byte[]> rawShares = new ArrayList<>();
    rawShares.add(null);
    rawShares.add(shamirSecretShares.getShareBytes(1));
    rawShares.add(shamirSecretShares.getShareBytes(2));
    rawShares.add(null);
    ShamirSecretShares readShares = ShamirSecretShares.FACTORY.fromSharesBytes(rawShares);

    Assertions.assertThat(readShares.reconstructPlainText()).isEqualTo(SECRET_PLAIN_TEXT);
  }

  /**
   * Test that plain-text can be reconstructed from {@link ShamirSecretShares} with a few enough
   * incorrect shares.
   */
  @Test
  public void reconstructShamirSecretSharesIncorrectShares() {
    ShamirSecretShares.ShamirFactory factory =
        new ShamirSecretShares.ShamirFactory(new ShamirSecretShares.ShamirConfig(2, 7, 5));
    ShamirSecretShares shamirSecretShares = factory.fromPlainText(7, SECRET_PLAIN_TEXT);

    List<byte[]> rawShares = new ArrayList<>();
    rawShares.add(new byte[SECRET_PLAIN_TEXT.length]);
    rawShares.add(shamirSecretShares.getShareBytes(1));
    rawShares.add(shamirSecretShares.getShareBytes(2));
    rawShares.add(shamirSecretShares.getShareBytes(3));
    rawShares.add(new byte[SECRET_PLAIN_TEXT.length]);
    rawShares.add(shamirSecretShares.getShareBytes(5));
    rawShares.add(shamirSecretShares.getShareBytes(6));
    ShamirSecretShares readShares = factory.fromSharesBytes(rawShares);

    Assertions.assertThat(readShares.reconstructPlainText()).isEqualTo(SECRET_PLAIN_TEXT);
  }

  /**
   * Unable to reconstruct the plain text from {@link ShamirSecretShares} with too many incorrect
   * shares.
   */
  @Test
  public void unableToReconstructShamirSecretShares() {
    ShamirSecretShares.ShamirFactory factory =
        new ShamirSecretShares.ShamirFactory(new ShamirSecretShares.ShamirConfig(2, 7, 5));
    ShamirSecretShares shamirSecretShares = factory.fromPlainText(7, SECRET_PLAIN_TEXT);

    List<byte[]> rawShares = new ArrayList<>();
    rawShares.add(new byte[SECRET_PLAIN_TEXT.length]);
    rawShares.add(shamirSecretShares.getShareBytes(1));
    rawShares.add(shamirSecretShares.getShareBytes(2));
    rawShares.add(new byte[SECRET_PLAIN_TEXT.length]);
    rawShares.add(new byte[SECRET_PLAIN_TEXT.length]);
    rawShares.add(shamirSecretShares.getShareBytes(5));
    rawShares.add(shamirSecretShares.getShareBytes(6));
    ShamirSecretShares readShares = factory.fromSharesBytes(rawShares);

    Assertions.assertThatThrownBy(readShares::reconstructPlainText)
        .isInstanceOf(RuntimeException.class)
        .hasMessage("Unable to reconstruct secret");
  }

  /** Multiplication for F256 is associative and commutative. */
  @Test
  void f256Multiply() {
    F256 a = F256.create(7);
    F256 b = F256.create(167);
    F256 c = F256.create(54);

    F256 x = a.multiply(b).multiply(c);
    F256 y = a.multiply(b.multiply(c));

    Assertions.assertThat(x).isEqualTo(y);
    Assertions.assertThat(a.multiply(b)).isEqualTo(b.multiply(a));
  }

  /** F256 has a multiplicative inverse with one as the identity. */
  @Test
  void f256Inverse() {
    for (int i = 1; i < 256; i++) {
      F256 a = F256.create(i);
      F256 b = a.modInverse();

      Assertions.assertThat(b.multiply(a)).isEqualTo(F256.ONE);
    }
  }
}
