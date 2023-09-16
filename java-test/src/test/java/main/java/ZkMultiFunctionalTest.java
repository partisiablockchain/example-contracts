package main.java;

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
          Path.of("../target/wasm32-unknown-unknown/release/zk_multi_functional.zkwa"),
          Path.of("../target/wasm32-unknown-unknown/release/zk_multi_functional.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/zk_multi_functional_contract_runner"));

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

  /** Contract state is updated to identity function of input variables. */
  @ContractTest(previous = "deploy")
  void identityFromInput() {
    final int secretValue = 42;
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(secretValue), new byte[] {0x40});

    ZkMultiFunctional.ContractState state = getState();

    // State now correctly holds the input value
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(secretValue);
  }

  /** Contract state is set to "4", when compute_4 is called. */
  @ContractTest(previous = "deploy")
  void produce_4() {
    byte[] compute4Rpc = ZkMultiFunctional.produce4();
    blockchain.sendAction(contractOwnerAccount, contractAddress, compute4Rpc);

    ZkMultiFunctional.ContractState state = getState();

    // After computation, the state is now correctly "4".
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(4);
  }

  /** State is correctly overwritten on multiple different computations. */
  @ContractTest(previous = "deploy")
  void computeMultiple() {
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(1337), new byte[] {0x40});

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
        contractAddress, contractOwnerAccount, createSecretInput(1337), new byte[] {0x40});
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(1338), new byte[] {0x40});
    blockchain.sendSecretInput(
        contractAddress, contractOwnerAccount, createSecretInput(1339), new byte[] {0x40});
    state = getState();
    Assertions.assertThat(state.latestProducedValue()).isEqualTo(1339);
  }

  private CompactBitArray createSecretInput(Integer secret) {
    return BitOutput.serializeBits(output -> output.writeSignedInt(secret, 32));
  }

  private ZkMultiFunctional.ContractState getState() {
    return ZkMultiFunctional.ContractState.deserialize(
        blockchain.getContractState(contractAddress));
  }
}
