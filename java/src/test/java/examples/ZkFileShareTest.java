package examples;

import com.fasterxml.jackson.databind.JsonNode;
import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkFileShare;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.Previous;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import java.util.Random;
import java.util.stream.Stream;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.junit.jupiter.params.provider.ValueSource;

/** Test suite for Zk File Sharing contract. */
public final class ZkFileShareTest extends JunitContractTest {

  private static final ContractBytes FILE_SHARE_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_file_share.zkwa"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_file_share.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_file_share_contract_runner"));

  private BlockchainAddress contractOwner;
  private BlockchainAddress initialUser;
  private BlockchainAddress secondUser;

  private BlockchainAddress fileShareAddress;

  private static final Random rand = new Random();

  /** Test that the contract can be correctly deployed. */
  @ContractTest
  void deploy() {
    contractOwner = blockchain.newAccount(17);
    initialUser = blockchain.newAccount(19);
    secondUser = blockchain.newAccount(23);

    byte[] initRpc = ZkFileShare.initialize();
    fileShareAddress = blockchain.deployZkContract(contractOwner, FILE_SHARE_BYTES, initRpc);

    ZkFileShare.CollectionState state =
        ZkFileShare.CollectionState.deserialize(blockchain.getContractState(fileShareAddress));

    Assertions.assertThat(state).isNotNull();
    assertSecretVariablesAmount(0);
  }

  /**
   * If a user tries to upload a file, and declares the correct file size, the upload succeeds, and
   * a new variable is added to the ZkState.
   */
  @Previous("deploy")
  @ParameterizedTest
  @MethodSource("fileAndGasSizes")
  void uploadFileWithSize(int fileSize, long gasAmount) {
    byte[] file = randomBytesOfLength(fileSize);
    CompactBitArray secretRpc = new CompactBitArray(file, fileSize * 8);

    blockchain.sendSecretInput(
        fileShareAddress, initialUser, secretRpc, publicRpc(fileSize), gasAmount);

    assertSecretVariablesAmount(1);
    assertSecretVariableOwner(1, initialUser);
  }

  /**
   * If a user declares the wrong file size when uploading, the invocation fails, and no variable is
   * added to ZkState.
   */
  @Previous("deploy")
  @ParameterizedTest
  @ValueSource(ints = {-1, -32, 1, 5, 8, 255})
  void uploadFileWrongSize(int delta) {
    final int fileSize = 1024;
    final int wrongSize = fileSize + delta;
    byte[] file = randomBytesOfLength(fileSize);
    CompactBitArray secretRpc = new CompactBitArray(file, fileSize * 8);

    Assertions.assertThatCode(
            () ->
                blockchain.sendSecretInput(
                    fileShareAddress, initialUser, secretRpc, publicRpc(wrongSize)))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining(
            "Expected an input of sizes [%d], but got sizes [%d]", wrongSize * 8, fileSize * 8);

    assertSecretVariablesAmount(0);
  }

  /**
   * The owner of a secret variable can change ownership, transferring the ownership of the file to
   * another user.
   */
  @ContractTest(previous = "uploadFileWithSize")
  void changeFileOwnership() {
    byte[] uploadFileRpc = ZkFileShare.changeFileOwner(1, secondUser);
    blockchain.sendAction(initialUser, fileShareAddress, uploadFileRpc);

    assertSecretVariablesAmount(1);
    assertSecretVariableOwner(1, secondUser);
  }

  /**
   * If a user tries to change ownership of a variable which they do not own, ownership doesn't
   * change.
   */
  @ContractTest(previous = "uploadFileWithSize")
  void changeFileOwnershipFail() {
    byte[] uploadFileRpc = ZkFileShare.changeFileOwner(1, secondUser);
    Assertions.assertThatCode(
            () -> blockchain.sendAction(secondUser, fileShareAddress, uploadFileRpc))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining("Only the owner of the secret file is allowed to change ownership.");

    assertSecretVariablesAmount(1);
    assertSecretVariableOwner(1, initialUser);
  }

  /** The owner of a variable can request its deletion, removing it from the contract. */
  @ContractTest(previous = "changeFileOwnership")
  void deleteOwnedFile() {
    byte[] deleteFileRpc = ZkFileShare.deleteFile(1);
    blockchain.sendAction(secondUser, fileShareAddress, deleteFileRpc);

    assertSecretVariablesAmount(0);
  }

  /**
   * If a non-owner tries to request a deletion of a secret variable, the request is rejected, and
   * the variable remains.
   */
  @ContractTest(previous = "changeFileOwnership")
  void deleteNonOwnedFile() {
    byte[] deleteFileRpc = ZkFileShare.deleteFile(1);
    Assertions.assertThatCode(
            () -> blockchain.sendAction(initialUser, fileShareAddress, deleteFileRpc))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining("Only the owner of the secret file is allowed to delete it.");

    assertSecretVariablesAmount(1);
    assertSecretVariableOwner(1, secondUser);
  }

  private Stream<Arguments> fileAndGasSizes() {
    return Stream.of(
        Arguments.arguments(0, 12_000),
        Arguments.arguments(1, 12_000),
        Arguments.arguments(255, 105_000),
        Arguments.arguments(256, 105_000),
        Arguments.arguments(333, 200_000),
        Arguments.arguments(727, 300_000),
        Arguments.arguments(1028, 385_000L),
        Arguments.arguments(1029, 385_000L),
        Arguments.arguments(1240, 460_000L));
  }

  private byte[] publicRpc(int fileSize) {
    return new byte[] {
      0x42,
      (byte) (fileSize >> 24),
      (byte) (fileSize >> 16),
      (byte) (fileSize >> 8),
      (byte) fileSize,
    };
  }

  private byte[] randomBytesOfLength(int length) {
    byte[] ret = new byte[length];
    rand.nextBytes(ret);
    return ret;
  }

  private void assertSecretVariablesAmount(int assertVarAmount) {
    final int realVarAmount =
        blockchain.getContractStateJson(fileShareAddress).getNode("/variables").size();

    Assertions.assertThat(realVarAmount).isEqualTo(assertVarAmount);
  }

  private void assertSecretVariableOwner(int variableId, BlockchainAddress assertOwner) {
    String realOwner = "";
    JsonNode variablesNode =
        blockchain.getContractStateJson(fileShareAddress).getNode("/variables");
    for (int i = 0; i < variablesNode.size(); i++) {
      final int id = variablesNode.get(i).get("value").get("id").asInt();
      if (id == variableId) {
        realOwner = variablesNode.get(i).get("value").get("owner").asText();
        break;
      }
    }

    String assertOwnerString = assertOwner.writeAsString();
    Assertions.assertThat(realOwner).isEqualTo(assertOwnerString);
  }
}
