package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.container.execution.protocol.Bytes;
import com.partisiablockchain.container.execution.protocol.HttpRequestData;
import com.partisiablockchain.container.execution.protocol.HttpResponseData;
import com.partisiablockchain.crypto.KeyPair;
import com.partisiablockchain.crypto.Signature;
import com.partisiablockchain.language.abicodegen.OffChainSecretSharing;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.testenvironment.executionengine.TestExecutionEngine;
import com.secata.tools.coverage.ExceptionConverter;
import examples.client.SecretSharingClient;
import examples.client.XorSecretShares;
import java.math.BigInteger;
import java.net.URI;
import java.nio.file.Path;
import java.util.List;
import org.assertj.core.api.Assertions;

/** Test suite for the {@link OffChainSecretSharing} contract. */
public final class OffChainSecretSharingClientTest extends JunitContractTest {

  /** Contract bytes for the {@link OffChainSecretSharing} contract. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/off_chain_secret_sharing.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/off_chain_secret_sharing_runner"));

  /** Addresses of the engines. */
  private List<OffChainSecretSharing.NodeConfig> engineConfigs;

  /** Engine test objects. */
  private List<TestExecutionEngine> engines;

  private final KeyPair senderKey = new KeyPair(BigInteger.TWO);
  private BlockchainAddress sender;
  private BlockchainAddress contractAddress;

  private static final BigInteger SHARING_ID_1 = BigInteger.ONE;
  private static final BigInteger SHARING_ID_2 = BigInteger.TWO;

  /** Deploys contracts and sets up execution engines. */
  @ContractTest
  void setup() {
    sender = blockchain.newAccount(senderKey);
    engines = OffChainSecretSharingTest.createEngines(blockchain);
    engineConfigs = OffChainSecretSharingTest.createEngineConfigs(blockchain);

    contractAddress =
        blockchain.deployContract(
            sender, CONTRACT_BYTES, OffChainSecretSharing.initialize(engineConfigs));
  }

  /** Client can upload sharings. */
  @ContractTest(previous = "setup")
  void registerAndUploadSharing() {
    secretSharingClient().registerAndUploadSharing(SHARING_ID_1, new byte[] {1, 2, 3, 4, 5});

    OffChainSecretSharing.ContractState state = getState();
    Assertions.assertThat(state.secretSharings().size()).isEqualTo(1);
    OffChainSecretSharing.Sharing sharing = state.secretSharings().get(SHARING_ID_1);
    Assertions.assertThat(sharing.owner()).isEqualTo(sender);
    Assertions.assertThat(sharing.nodesWithCompletedUpload())
        .isEqualTo(List.of(true, true, true, true));
  }

  /** Client can download and reconstruct the original plaintext. */
  @ContractTest(previous = "registerAndUploadSharing")
  void downloadAndReconstruct() {
    final byte[] result = secretSharingClient().downloadAndReconstruct(SHARING_ID_1);
    Assertions.assertThat(result).containsExactly(1, 2, 3, 4, 5);
  }

  /** Multiple secret shares can be uploaded and not interfer with each other. */
  @ContractTest(previous = "registerAndUploadSharing")
  void multipleSecretSharesUploadedAndNotInterferWithEachOther() {
    secretSharingClient().registerAndUploadSharing(SHARING_ID_2, new byte[] {5, 4, 3, 2, 1});

    Assertions.assertThat(secretSharingClient().downloadAndReconstruct(SHARING_ID_1))
        .containsExactly(1, 2, 3, 4, 5);
    Assertions.assertThat(secretSharingClient().downloadAndReconstruct(SHARING_ID_2))
        .containsExactly(5, 4, 3, 2, 1);
  }

  /** Cannot overwrite existing sharing. */
  @ContractTest(previous = "registerAndUploadSharing")
  void cannotOverwriteExistingSharing() {
    Assertions.assertThatThrownBy(
            () ->
                secretSharingClient()
                    .registerAndUploadSharing(SHARING_ID_1, new byte[] {5, 4, 3, 2, 1}))
        .hasMessageContaining("Cannot register sharing with the same identifier");
  }

  /** The engine fails when getting share before it has been uploaded. */
  @ContractTest(previous = "registerAndUploadSharing")
  void engineFailsWhenGettingShareBeforeHasBeenUploaded() {
    Assertions.assertThatThrownBy(() -> secretSharingClient().downloadAndReconstruct(SHARING_ID_2))
        .hasMessageContaining("No such sharing");
  }

  private OffChainSecretSharing.ContractState getState() {
    return getOffChainSecretSharing().getState();
  }

  private SecretSharingClient<XorSecretShares> secretSharingClient() {
    return SecretSharingClient.forTestBlockchain(
        (BlockchainAddress contractAddress, byte[] payload, long gasCost) ->
            blockchain.sendAction(sender, contractAddress, payload, gasCost),
        contractAddress,
        senderKey,
        XorSecretShares.FACTORY,
        getOffChainSecretSharing(),
        new DelegateHttpToEngines(),
        () -> blockchain.getBlockProductionTime());
  }

  private final class DelegateHttpToEngines implements SecretSharingClient.EndpointHttpClient {

    @Override
    public byte[] downloadShare(Signature signature, String fullUrl, long timestamp) {
      final URI uri = ExceptionConverter.call(() -> new URI(fullUrl), "Bad url");
      final String contractPath = uri.getPath().substring(52);

      HttpRequestData requestData =
          new HttpRequestData(
              "GET",
              contractPath,
              OffChainSecretSharingTest.createHeaders(signature, timestamp),
              Bytes.EMPTY);

      final int engineIndex = engineIndexForHostname(uri.getHost());
      final HttpResponseData response =
          engines.get(engineIndex).makeHttpRequest(contractAddress, requestData).response();

      Assertions.assertThat(response.statusCode()).isEqualTo(200);

      return response.body().data();
    }

    @Override
    public int uploadShare(
        Signature signature, String fullUrl, long timestamp, byte[] secretShare) {
      final URI uri = ExceptionConverter.call(() -> new URI(fullUrl), "Bad url");
      final String contractPath = uri.getPath().substring(52);

      HttpRequestData requestData =
          new HttpRequestData(
              "PUT",
              contractPath,
              OffChainSecretSharingTest.createHeaders(signature, timestamp),
              Bytes.fromBytes(secretShare));

      final int engineIndex = engineIndexForHostname(uri.getHost());
      final HttpResponseData response =
          engines.get(engineIndex).makeHttpRequest(contractAddress, requestData).response();
      return response.statusCode();
    }

    private int engineIndexForHostname(String hostname) {
      for (int engineIndex = 0; engineIndex < engineConfigs.size(); engineIndex++) {
        if (engineConfigs.get(engineIndex).endpoint().contains(hostname)) {
          return engineIndex;
        }
      }
      return -1;
    }
  }

  private OffChainSecretSharing getOffChainSecretSharing() {
    return new OffChainSecretSharing(getStateClient(), contractAddress);
  }
}
