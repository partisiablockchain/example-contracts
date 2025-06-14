package examples;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.Nickname;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import java.nio.file.Path;
import java.util.HexFormat;

/** Test suite for the Nickname contract. */
public final class NicknameTest extends JunitContractTest {
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/nickname.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/nickname_runner"));
  private BlockchainAddress account;
  private BlockchainAddress nicknameAddress;
  private Nickname nicknameContract;

  /** Setup for all the other tests. Deploys the contract and gives a nickname. */
  @ContractTest
  void setup() {
    account = blockchain.newAccount(2);
    byte[] initRpc = Nickname.initialize();
    nicknameAddress = blockchain.deployContract(account, CONTRACT_BYTES, initRpc);
    nicknameContract = new Nickname(getStateClient(), nicknameAddress);

    BlockchainAddress address =
        BlockchainAddress.fromString("000000000000000000000000000000000000000001");
    String nickname = "My nickname";

    byte[] rpc = Nickname.giveNickname(address, nickname);
    blockchain.sendAction(account, nicknameAddress, rpc);

    Nickname.ContractState state = nicknameContract.getState();
    assertThat(state.nicknames().treeId()).isEqualTo(0);
    assertThat(state.nicknames().get(address)).isEqualTo(nickname);
  }

  /** Can give a nickname to an address. */
  @ContractTest(previous = "setup")
  void giveNickname() {
    BlockchainAddress address =
        BlockchainAddress.fromString("000000000000000000000000000000000000000002");
    String nickname = "abc";
    byte[] rpc = Nickname.giveNickname(address, nickname);
    blockchain.sendAction(account, nicknameAddress, rpc);

    Nickname.ContractState state = nicknameContract.getState();
    assertThat(state.nicknames().get(address)).isEqualTo(nickname);
  }

  /** Can overwrite an existing nickname with a new nickname. */
  @ContractTest(previous = "setup")
  void overwriteNickname() {
    BlockchainAddress address =
        BlockchainAddress.fromString("000000000000000000000000000000000000000001");
    String nickname = "new nickname";
    byte[] rpc = Nickname.giveNickname(address, nickname);
    blockchain.sendAction(account, nicknameAddress, rpc);

    Nickname.ContractState state = nicknameContract.getState();
    assertThat(state.nicknames().get(address)).isEqualTo(nickname);
  }

  /** Can remove a nickname from an address. */
  @ContractTest(previous = "setup")
  void removeNickname() {
    BlockchainAddress address =
        BlockchainAddress.fromString("000000000000000000000000000000000000000001");
    byte[] rpc = Nickname.removeNickname(address);
    blockchain.sendAction(account, nicknameAddress, rpc);

    Nickname.ContractState state = nicknameContract.getState();
    assertThat(state.nicknames().getNextN(null, 10).size()).isEqualTo(0);
  }

  /** Removing nonexistent nickname has no effect. */
  @ContractTest(previous = "setup")
  void removeNonexistentNickname() {
    BlockchainAddress address =
        BlockchainAddress.fromString("000000000000000000000000000000000000000042");
    byte[] rpc = Nickname.removeNickname(address);
    blockchain.sendAction(account, nicknameAddress, rpc);

    Nickname.ContractState state = nicknameContract.getState();
    assertThat(
            state
                .nicknames()
                .get(BlockchainAddress.fromString("000000000000000000000000000000000000000001")))
        .isEqualTo("My nickname");
  }

  /** A failing transaction doesn't update the nicknames. */
  @ContractTest(previous = "setup")
  void failingTransaction() {
    BlockchainAddress address =
        BlockchainAddress.fromString("000000000000000000000000000000000000000002");
    String nickname = "abc";
    byte[] rpc = Nickname.giveNickname(address, nickname);
    assertThatThrownBy(() -> blockchain.sendAction(account, nicknameAddress, rpc, 900))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Ran out of gas");

    Nickname.ContractState state = nicknameContract.getState();
    assertThat(state.nicknames().getNextN(null, 10).size()).isEqualTo(1);
  }

  /** Can handle many nicknames. */
  @ContractTest(previous = "setup")
  void manyNicknames() {
    for (int i = 0; i < 1000; i++) {
      String nickname = HexFormat.of().toHexDigits(i);
      BlockchainAddress address = BlockchainAddress.fromString("00".repeat(17) + nickname);
      blockchain.sendAction(account, nicknameAddress, Nickname.giveNickname(address, nickname));
    }
    Nickname.ContractState state = nicknameContract.getState();
    for (int i = 0; i < 1000; i++) {
      String nickname = HexFormat.of().toHexDigits(i);
      BlockchainAddress address = BlockchainAddress.fromString("00".repeat(17) + nickname);
      assertThat(state.nicknames().get(address)).isEqualTo(nickname);
    }
  }
}
