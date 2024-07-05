package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkMultiFunctional;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;

/** {@link ZkMultiFunctional} testing. */
public final class ZkMultiFunctionalTest extends JunitContractTest {

  /** {@link ZkMultiFunctional} contract bytes. */
  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_multi_functional.zkwa"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_multi_functional.abi"),
          Path.of(
              "../rust/target/wasm32-unknown-unknown/release/zk_multi_functional_contract_runner"));

  private BlockchainAddress contractOwnerAccount;

  private BlockchainAddress contractAddress;

  /** Contract can be correctly deployed. */
  @ContractTest
  void deploy() {
    contractOwnerAccount = blockchain.newAccount(2);

    byte[] initRpc = ZkMultiFunctional.initialize();
    contractAddress = blockchain.deployZkContract(contractOwnerAccount, CONTRACT_BYTES, initRpc);

    ZkMultiFunctional.ContractState state = getState();

    // State correctly initialized
    Assertions.assertThat(state).isNotNull();
    Assertions.assertThat(state.latestProducedValue()).isNull();
  }

  /**
   * A user can send a secret input, which the contract state holds as its latest produced value.
   */
  @ContractTest(previous = "deploy")
  void identityFromInput() {
    final int secretValue = 42;
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(secretValue), secretInputRpc());

    ZkMultiFunctional.ContractState state = getState();

    // State now correctly holds the input value
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(secretValue);
  }

  /** A user can set the latest produced value to "4" by calling compute_4. */
  @ContractTest(previous = "deploy")
  void produce_4() {
    byte[] compute4Rpc = ZkMultiFunctional.produce4();
    blockchain.sendAction(contractOwnerAccount, contractAddress, compute4Rpc);

    ZkMultiFunctional.ContractState state = getState();

    // After computation, the state is now correctly "4".
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(4);
  }

  /**
   * A user can overwrite the latest produced value in the state by sending multiple computations.
   */
  @ContractTest(previous = "deploy")
  void computeMultiple() {
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(1337), secretInputRpc());

    ZkMultiFunctional.ContractState state = getState();

    // State is correctly initialized.
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(1337);

    // Contract state is overwritten with "4".
    byte[] compute4Rpc = ZkMultiFunctional.produce4();
    blockchain.sendAction(contractOwnerAccount, contractAddress, compute4Rpc);
    state = getState();
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(4);

    // After multiple computations, the contract state holds the latest one.
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(1337), secretInputRpc());
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(1338), secretInputRpc());
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(1339), secretInputRpc());
    state = getState();
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(1339);
  }

  private CompactBitArray createSecretInput(Integer secret) {
    return BitOutput.serializeBits(output -> output.writeSignedInt(secret, 32));
  }

  byte[] secretInputRpc() {
    return new byte[] {0x40};
  }

  private ZkMultiFunctional.ContractState getState() {
    return ZkMultiFunctional.ContractState.deserialize(
        blockchain.getContractState(contractAddress));
  }
}
