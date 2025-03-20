package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.crypto.Hash;
import com.partisiablockchain.language.abicodegen.UpgradableV1;
import com.partisiablockchain.language.abicodegen.UpgradableV2;
import com.partisiablockchain.language.abicodegen.UpgradableV3;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.language.testenvironment.dependencies.GovernanceId;
import com.partisiablockchain.language.testenvironment.dependencies.GovernanceLoader;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.api.DisplayName;

/**
 * Test suite for upgradable smart contracts.
 *
 * <p>This test suit checks that {@link UpgradableV1} can be upgraded to {@link UpgradableV2}, and
 * then further upgraded to {@link UpgradableV3}.
 */
public final class UpgradableTest extends JunitContractTest {

  /** {@link UpgradableV1} contract bytes. * */
  private static final ContractBytes CONTRACT_BYTES_V1 =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/upgradable_v1.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/upgradable_v1_runner"));

  /** {@link UpgradableV2} contract bytes. * */
  private static final ContractBytes CONTRACT_BYTES_V2 =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/upgradable_v2.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/upgradable_v2_runner"));

  /** {@link UpgradableV3} contract bytes. * */
  private static final ContractBytes CONTRACT_BYTES_V3 =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/upgradable_v3.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/upgradable_v3_runner"));

  private BlockchainAddress upgrader;
  private BlockchainAddress upgradableContract;

  @DisplayName("Upgradable V1 can be deployed")
  @ContractTest
  void deployV1() {
    upgrader = blockchain.newAccount(1);
    byte[] initRpc = UpgradableV1.initialize(upgrader);

    upgradableContract = blockchain.deployContract(upgrader, CONTRACT_BYTES_V1, initRpc);

    // Get the main contract's state.
    UpgradableV1.ContractState state =
        UpgradableV1.ContractState.deserialize(blockchain.getContractState(upgradableContract));

    // Contract state is correctly initialized.
    Assertions.assertThat(state).isNotNull();
    Assertions.assertThat(state.upgrader()).isEqualTo(upgrader);
    Assertions.assertThat(state.counter()).isEqualTo(0);
  }

  @DisplayName("Upgradable V1s counter is incremented by one")
  @ContractTest(previous = "deployV1")
  void incrementV1byOne() {
    byte[] incrRpc = UpgradableV1.incrementCounterByOne();
    blockchain.sendAction(upgrader, upgradableContract, incrRpc);

    // Get the main contract's state.
    UpgradableV1.ContractState state =
        UpgradableV1.ContractState.deserialize(blockchain.getContractState(upgradableContract));
    Assertions.assertThat(state).isNotNull();
    Assertions.assertThat(state.counter()).isEqualTo(1);
  }

  @DisplayName("Upgradable V1 can be upgraded to V2")
  @ContractTest(previous = "incrementV1byOne")
  void upgradeV1ToV2() {
    blockchain.upgradeContract(upgrader, upgradableContract, CONTRACT_BYTES_V2, new byte[0]);

    // Get the main contract's state.
    UpgradableV2.ContractState state =
        UpgradableV2.ContractState.deserialize(blockchain.getContractState(upgradableContract));
    Assertions.assertThat(state.upgradableTo()).isNull();
    Assertions.assertThat(state.counter()).isEqualTo(1); // Counter should still be one
  }

  @DisplayName("Upgradable V1 cannot be upgraded to V1")
  @ContractTest(previous = "deployV1")
  void cannotUpgradeFromV1ToV1() {
    Assertions.assertThatThrownBy(
            () ->
                blockchain.upgradeContract(
                    upgrader, upgradableContract, CONTRACT_BYTES_V1, new byte[0]))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "Contract does not implement \"upgrade\", and can thus not be upgraded to");
  }

  @DisplayName("Non-upgraders cannot upgrade V1")
  @ContractTest(previous = "deployV1")
  void onlyUpgraderCanUpgrade() {
    BlockchainAddress user2 = blockchain.newAccount(2);
    Assertions.assertThatThrownBy(
            () ->
                blockchain.upgradeContract(
                    user2, upgradableContract, CONTRACT_BYTES_V2, new byte[0]))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining("Contract did not allow this upgrade");
  }

  @DisplayName("Upgradable V2's logic replaced V1's")
  @ContractTest(previous = "upgradeV1ToV2")
  void incrementV2ByTwo() {
    byte[] incrRpc = UpgradableV2.incrementCounterByTwo();
    blockchain.sendAction(upgrader, upgradableContract, incrRpc);

    UpgradableV2.ContractState state =
        UpgradableV2.ContractState.deserialize(blockchain.getContractState(upgradableContract));
    Assertions.assertThat(state.upgradableTo()).isNull();
    Assertions.assertThat(state.counter()).isEqualTo(3);
  }

  @DisplayName("Upgrader can propose upgrade from V2 to V3")
  @ContractTest(previous = "incrementV2ByTwo")
  void upgraderCanProposeUpgrade() {
    byte[] upgradeRpc = UpgradableV2.allowUpgradeTo(contractHashes(CONTRACT_BYTES_V3), new byte[0]);

    // Upgrader proposes an upgrade to V3
    blockchain.sendAction(upgrader, upgradableContract, upgradeRpc);

    UpgradableV2.ContractState state =
        UpgradableV2.ContractState.deserialize(blockchain.getContractState(upgradableContract));
    Assertions.assertThat(state.upgradableTo()).isNotNull();
  }

  @DisplayName("Upgradable V2 can be upgraded to to V3 if proposed")
  @ContractTest(previous = "upgraderCanProposeUpgrade")
  void upgradeV2ToV3() {
    BlockchainAddress user = blockchain.newAccount(2);
    blockchain.upgradeContract(user, upgradableContract, CONTRACT_BYTES_V3, new byte[0]);

    UpgradableV3.ContractState state =
        UpgradableV3.ContractState.deserialize(blockchain.getContractState(upgradableContract));
    Assertions.assertThat(state).isNotNull();
    Assertions.assertThat(state.counter()).isEqualTo(3);
  }

  @DisplayName("Upgradable V3's logic replaced V2's")
  @ContractTest(previous = "upgradeV2ToV3")
  void incrementV3ByFour() {
    UpgradableV3.ContractState state =
        UpgradableV3.ContractState.deserialize(blockchain.getContractState(upgradableContract));
    Assertions.assertThat(state.counter()).isEqualTo(3);

    byte[] incrRpc = UpgradableV3.incrementCounter();
    blockchain.sendAction(upgrader, upgradableContract, incrRpc);

    state = UpgradableV3.ContractState.deserialize(blockchain.getContractState(upgradableContract));
    Assertions.assertThat(state).isNotNull();
    Assertions.assertThat(state.counter()).isEqualTo(7);
  }

  @DisplayName("V3 cannot be upgraded")
  @ContractTest(previous = "upgradeV2ToV3")
  void contractV3CannotBeUpgraded() {
    Assertions.assertThatThrownBy(
            () ->
                blockchain.upgradeContract(
                    upgrader, upgradableContract, CONTRACT_BYTES_V2, new byte[0]))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "Contract does not implement \"upgrade_is_allowed\", and defaults to not upgradable");
  }

  @DisplayName("Non-upgraders cannot propose upgrades")
  @ContractTest(previous = "upgradeV1ToV2")
  void nonUpgradersCannotProposeUpgrade() {
    final Hash dummyHash =
        Hash.create(
            s -> {
              s.writeInt(42);
            });
    UpgradableV2.ContractHashes contractHash =
        new UpgradableV2.ContractHashes(dummyHash, dummyHash, dummyHash);
    byte[] upgradeRpc = UpgradableV2.allowUpgradeTo(contractHash, new byte[0]);
    // Upgrader proposes an upgrade to V3
    BlockchainAddress user = blockchain.newAccount(123);

    Assertions.assertThatThrownBy(() -> blockchain.sendAction(user, upgradableContract, upgradeRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "The upgrade_proposer is the only address allowed to propose upgrades");
  }

  @DisplayName("Upgrade fails V2 to V3 if the contract does not match proposal")
  @ContractTest(previous = "upgradeV1ToV2")
  void cannotUpgradeWithNoProposal() {
    BlockchainAddress user = blockchain.newAccount(2);
    Assertions.assertThatThrownBy(
            () ->
                blockchain.upgradeContract(
                    user, upgradableContract, CONTRACT_BYTES_V3, new byte[0]))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Given contract code does not match approved hashes!");
  }

  @DisplayName("V2 cannot be upgraded to V1")
  @ContractTest(previous = "upgradeV1ToV2")
  void contractV2CannotUpgradeToV1() {
    // Make a valid proposal
    byte[] upgradeRpc = UpgradableV2.allowUpgradeTo(contractHashes(CONTRACT_BYTES_V2), new byte[0]);
    // Upgrader proposes an upgrade to V1
    blockchain.sendAction(upgrader, upgradableContract, upgradeRpc);

    Assertions.assertThatThrownBy(
            () ->
                blockchain.upgradeContract(
                    upgrader, upgradableContract, CONTRACT_BYTES_V2, new byte[0]))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "State type witness from RPC was not consistent with that from the state object. This"
                + " indicates that the definitions for the old contract state (in the new contract)"
                + " does not match the definitions as they were in the old contract!");
  }

  /**
   * {@link UpgradableV2.ContractHashes} for upgrading to the contract code given by {@link
   * ContractBytes}.
   *
   * @param contractBytes Bytecode to upgrade to. Not nullable.
   * @return Contract hashes for the upgrade. Not nullable.
   */
  private UpgradableV2.ContractHashes contractHashes(ContractBytes contractBytes) {
    return new UpgradableV2.ContractHashes(
        Hash.create(s -> s.writeDynamicBytes(contractBytes.code())),
        getPubWasmBinderHash(),
        Hash.create(s -> s.writeDynamicBytes(contractBytes.abi())));
  }

  /**
   * Get {@link Hash} of the binder for public WASM contracts.
   *
   * @return {@link Hash} of the binder for public WASM contracts. Not nullable.
   */
  private Hash getPubWasmBinderHash() {
    final GovernanceLoader governanceLoader =
        GovernanceLoader.createGovernanceLoaderFromProperties();
    final byte[] byteCode = governanceLoader.getByteCode(GovernanceId.PUB_BINDER);
    return Hash.create(s -> s.writeDynamicBytes(byteCode));
  }
}
