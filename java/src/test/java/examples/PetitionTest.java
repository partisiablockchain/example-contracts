package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.Petition;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;

/** Test suite for the Petition smart contract. */
public final class PetitionTest extends JunitContractTest {

  /** {@link Petition} contract bytes. */
  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/petition.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/petition.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/petition_runner"));

  private BlockchainAddress petition;
  private BlockchainAddress owner;
  private BlockchainAddress signer1;
  private BlockchainAddress signer2;

  /** Contract can be correctly deployed. */
  @ContractTest
  void deploy() {
    owner = blockchain.newAccount(1);
    signer1 = blockchain.newAccount(2);
    signer2 = blockchain.newAccount(3);

    String description = "I love cake!";
    byte[] initRpc = Petition.initialize(description);

    petition = blockchain.deployContract(owner, CONTRACT_BYTES, initRpc);

    Petition.PetitionState state =
        Petition.PetitionState.deserialize(blockchain.getContractState(petition));

    Assertions.assertThat(state.description()).isEqualTo(description);
    Assertions.assertThat(state.signedBy()).isEmpty();
  }

  /** Petitions cannot be created without a description. */
  @ContractTest
  void deployWithEmptyDescription() {
    owner = blockchain.newAccount(1);
    signer1 = blockchain.newAccount(2);
    signer2 = blockchain.newAccount(3);

    String description = "";
    byte[] initRpc = Petition.initialize(description);

    Assertions.assertThatThrownBy(() -> blockchain.deployContract(owner, CONTRACT_BYTES, initRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("The description af a petition cannot be empty.");
  }

  /** Signing the petition is registered. */
  @ContractTest(previous = "deploy")
  void sign() {
    byte[] sign = Petition.sign();
    blockchain.sendAction(signer1, petition, sign);

    Petition.PetitionState state =
        Petition.PetitionState.deserialize(blockchain.getContractState(petition));

    Assertions.assertThat(state.signedBy().size()).isEqualTo(1);
    Assertions.assertThat(state.signedBy()).contains(signer1);
  }

  /** Owner can also sign the petition. */
  @ContractTest(previous = "deploy")
  void ownersSign() {
    byte[] sign = Petition.sign();
    blockchain.sendAction(owner, petition, sign);

    Petition.PetitionState state =
        Petition.PetitionState.deserialize(blockchain.getContractState(petition));

    Assertions.assertThat(state.signedBy().size()).isEqualTo(1);
    Assertions.assertThat(state.signedBy()).contains(owner);
  }

  /** Different users signing the petition is registered. */
  @ContractTest(previous = "deploy")
  void signedByDifferentUsers() {
    byte[] sign = Petition.sign();
    blockchain.sendAction(signer1, petition, sign);
    blockchain.sendAction(signer2, petition, sign);

    Petition.PetitionState state =
        Petition.PetitionState.deserialize(blockchain.getContractState(petition));

    Assertions.assertThat(state.signedBy().size()).isEqualTo(2);
    Assertions.assertThat(state.signedBy()).contains(signer1);
    Assertions.assertThat(state.signedBy()).contains(signer2);
  }

  /** Signing the petition twice, is equivalent to signing it once. */
  @ContractTest(previous = "deploy")
  void signTwice() {
    byte[] sign = Petition.sign();

    // Sign twice by signer1
    blockchain.sendAction(signer1, petition, sign);
    blockchain.sendAction(signer1, petition, sign);

    Petition.PetitionState state =
        Petition.PetitionState.deserialize(blockchain.getContractState(petition));

    Assertions.assertThat(state.signedBy().size()).isEqualTo(1);
    Assertions.assertThat(state.signedBy()).contains(signer1);
  }
}
