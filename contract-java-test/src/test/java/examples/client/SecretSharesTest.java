package examples.client;

import static java.nio.charset.StandardCharsets.UTF_8;

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
    Assertions.assertThat(
            ShamirSecretShares.FACTORY.fromPlainText(4, SECRET_PLAIN_TEXT).reconstructPlainText())
        .isEqualTo(SECRET_PLAIN_TEXT);
  }
}
