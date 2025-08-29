package examples.client;

import com.partisiablockchain.crypto.Hash;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.Test;

/** Testing {@link SecretSharingClient}. */
public final class SecretSharingClientTest {

  /** Shares whose hash does not match its expected commitment are filtered away. */
  @Test
  void filterSharesFromCommitments() {
    byte[] invalidShare = "invalidShare".getBytes(StandardCharsets.UTF_8);
    byte[] validShare = "validShare".getBytes(StandardCharsets.UTF_8);

    List<Hash> expectedCommitments =
        List.of(
            Hash.create(stream -> stream.writeString("correctFirstShare")),
            Hash.create(stream -> stream.write(validShare)),
            Hash.create(stream -> stream.writeString("missingShare")));

    List<byte[]> shares = new ArrayList<>();
    shares.add(invalidShare);
    shares.add(validShare);
    shares.add(null);

    List<byte[]> filteredShares =
        SecretSharingClient.filterSharesFromCommitments(expectedCommitments, shares);
    Assertions.assertThat(filteredShares).hasSize(3);
    Assertions.assertThat(filteredShares.get(0)).isNull();
    Assertions.assertThat(filteredShares.get(1)).isEqualTo(validShare);
    Assertions.assertThat(filteredShares.get(2)).isNull();
  }
}
