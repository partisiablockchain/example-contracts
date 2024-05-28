package examples;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.AccessControl;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import java.nio.file.Path;

/** Tests. */
public final class AccessControlTest extends JunitContractTest {
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/access_control.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/access_control.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/access_control_runner"));
  private BlockchainAddress account1;
  private BlockchainAddress account2;
  private BlockchainAddress account3;
  private BlockchainAddress account4;
  private BlockchainAddress accessControlContract;

  /** Setup for all the other tests. Deploys the contract. */
  @ContractTest
  void setup() {
    account1 = blockchain.newAccount(1);
    account2 = blockchain.newAccount(2);
    account3 = blockchain.newAccount(3);
    account4 = blockchain.newAccount(4);
    byte[] initRpc = AccessControl.initialize("My favourite book");
    accessControlContract = blockchain.deployContract(account1, CONTRACT_BYTES, initRpc);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.accessMap().map()).hasSize(1);
    assertThat(state.accessMap().map().get(account1).discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.ADMIN);
    assertThat(state.description().data()).isEqualTo("My favourite book");
    assertThat(state.description().level().discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.ADMIN);
    assertThat(state.currentlyHeldBy().data()).isEqualTo(null);
    assertThat(state.currentlyHeldBy().level().discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.USER);
  }

  /** User can borrow object. */
  @ContractTest(previous = "setup")
  void userCanBorrow() {
    byte[] payload = AccessControl.borrowObject();
    blockchain.sendAction(account2, accessControlContract, payload);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.currentlyHeldBy().data()).isEqualTo(account2);
  }

  /** Only the user who borrowed the object can return it borrow object. */
  @ContractTest(previous = "userCanBorrow")
  void returningTheObject() {
    byte[] payload = AccessControl.returnObject();
    assertThatThrownBy(() -> blockchain.sendAction(account1, accessControlContract, payload))
        .hasMessageContaining("Only the user who has borrowed the object can return it");
    assertThatThrownBy(() -> blockchain.sendAction(account3, accessControlContract, payload))
        .hasMessageContaining("Only the user who has borrowed the object can return it");
    assertThatThrownBy(() -> blockchain.sendAction(account4, accessControlContract, payload))
        .hasMessageContaining("Only the user who has borrowed the object can return it");

    blockchain.sendAction(account2, accessControlContract, payload);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.currentlyHeldBy().data()).isEqualTo(null);
  }

  /** Can't return the object when it is not lent out. */
  @ContractTest(previous = "setup")
  void cantReturnWhenNotLent() {
    byte[] payload = AccessControl.returnObject();
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload))
        .hasMessageContaining("Only the user who has borrowed the object can return it");

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.currentlyHeldBy().data()).isEqualTo(null);
  }

  /** User cannot update description, but admin can. */
  @ContractTest(previous = "setup")
  void userCannotUpdateDescription() {
    byte[] payload = AccessControl.updateDescription("New description");
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload))
        .hasMessageContaining(
            "User with level 'User' does not have the privilege to update data with level 'Admin'");

    blockchain.sendAction(account1, accessControlContract, payload);
    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.description().data()).isEqualTo("New description");
  }

  /** Update description level to user, such that a user can update the string. */
  @ContractTest(previous = "setup")
  void updateDescriptionLevel() {
    byte[] payload =
        AccessControl.updateDescriptionLevel(new AccessControl.SecurityLevelImpl.User());
    blockchain.sendAction(account1, accessControlContract, payload);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.description().level().discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.USER);

    payload = AccessControl.updateDescription("New low level description");
    blockchain.sendAction(account2, accessControlContract, payload);

    state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.description().data()).isEqualTo("New low level description");
  }

  /** Update borrow level to ModeratorA. Now users cannot update borrow. */
  @ContractTest(previous = "setup")
  void updateBorrowLevel() {
    byte[] payload =
        AccessControl.updateBorrowLevel(new AccessControl.SecurityLevelImpl.ModeratorA());
    blockchain.sendAction(account1, accessControlContract, payload);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.currentlyHeldBy().level().discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.MODERATOR_A);

    byte[] payload2 = AccessControl.borrowObject();
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload2))
        .hasMessageContaining(
            "User with level 'User' does not have the privilege to update data with level"
                + " 'ModeratorA'");
  }

  /** Update user levels. */
  @ContractTest(previous = "updateBorrowLevel")
  void updateUserLevels() {
    byte[] payload =
        AccessControl.updateUserLevel(account2, new AccessControl.SecurityLevelImpl.ModeratorA());
    blockchain.sendAction(account1, accessControlContract, payload);

    payload =
        AccessControl.updateUserLevel(account3, new AccessControl.SecurityLevelImpl.ModeratorB());
    blockchain.sendAction(account1, accessControlContract, payload);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.accessMap().map()).hasSize(3);
    assertThat(state.accessMap().map().get(account2).discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.MODERATOR_A);
    assertThat(state.accessMap().map().get(account3).discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.MODERATOR_B);
  }

  /** Only moderatorA and Admin can update borrow now. */
  @ContractTest(previous = "updateUserLevels")
  void updateBorrowDataModeratorA() {
    byte[] payload3 = AccessControl.borrowObject();
    assertThatThrownBy(() -> blockchain.sendAction(account3, accessControlContract, payload3))
        .hasMessageContaining(
            "User with level 'ModeratorB' does not have the privilege to update data with level"
                + " 'ModeratorA'");

    byte[] payload4 = AccessControl.borrowObject();
    assertThatThrownBy(() -> blockchain.sendAction(account4, accessControlContract, payload4))
        .hasMessageContaining(
            "User with level 'User' does not have the privilege to update data with level"
                + " 'ModeratorA'");

    byte[] payload = AccessControl.borrowObject();
    blockchain.sendAction(account1, accessControlContract, payload);
    blockchain.sendAction(account1, accessControlContract, AccessControl.returnObject());

    byte[] payload2 = AccessControl.borrowObject();
    blockchain.sendAction(account2, accessControlContract, payload2);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.currentlyHeldBy().data()).isEqualTo(account2);
  }

  /** Moderator can update Users' level. */
  @ContractTest(previous = "updateUserLevels")
  void moderatorUpdateUserLevel() {
    byte[] payload =
        AccessControl.updateUserLevel(account4, new AccessControl.SecurityLevelImpl.ModeratorA());
    blockchain.sendAction(account2, accessControlContract, payload);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.accessMap().map().get(account4).discriminant())
        .isEqualTo(AccessControl.SecurityLevelImplD.MODERATOR_A);
  }

  /** Cannot update levels of users with level not lower than own. */
  @ContractTest(previous = "updateUserLevels")
  void cannotUpdateHigherUsers() {
    byte[] payload =
        AccessControl.updateUserLevel(account1, new AccessControl.SecurityLevelImpl.User());
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload))
        .hasMessageContaining("Sender level 'ModeratorA' cannot update user with level 'Admin'");

    byte[] payload2 =
        AccessControl.updateUserLevel(account3, new AccessControl.SecurityLevelImpl.ModeratorA());
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload2))
        .hasMessageContaining(
            "Sender level 'ModeratorA' cannot update user with level 'ModeratorB'");
  }

  /** Cannot update users to levels not lower or equal to own. */
  @ContractTest(previous = "updateUserLevels")
  void cannotUpdateToHigherLevel() {
    byte[] payload =
        AccessControl.updateUserLevel(account4, new AccessControl.SecurityLevelImpl.ModeratorB());
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload))
        .hasMessageContaining(
            "Sender level 'ModeratorA' cannot update user to new level 'ModeratorB'");

    byte[] payload2 =
        AccessControl.updateUserLevel(account4, new AccessControl.SecurityLevelImpl.Admin());
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload2))
        .hasMessageContaining("Sender level 'ModeratorA' cannot update user to new level 'Admin'");
  }

  /** Non-admins cannot update levels. */
  @ContractTest(previous = "updateUserLevels")
  void nonAdminUpdateLevel() {
    byte[] payload =
        AccessControl.updateDescriptionLevel(new AccessControl.SecurityLevelImpl.User());
    assertThatThrownBy(() -> blockchain.sendAction(account2, accessControlContract, payload))
        .hasMessageContaining("Only 'Admin' can update level");

    byte[] payload2 =
        AccessControl.updateBorrowLevel(new AccessControl.SecurityLevelImpl.ModeratorA());
    assertThatThrownBy(() -> blockchain.sendAction(account3, accessControlContract, payload2))
        .hasMessageContaining("Only 'Admin' can update level");

    byte[] payload3 =
        AccessControl.updateBorrowLevel(new AccessControl.SecurityLevelImpl.ModeratorA());
    assertThatThrownBy(() -> blockchain.sendAction(account4, accessControlContract, payload3))
        .hasMessageContaining("Only 'Admin' can update level");
  }

  /** User cannot borrow an object, that is already borrowed. */
  @ContractTest(previous = "setup")
  void objectBorrowTwiceByDifferentUsers() {
    byte[] payload = AccessControl.borrowObject();
    blockchain.sendAction(account2, accessControlContract, payload);

    AccessControl.ContractState state =
        AccessControl.ContractState.deserialize(blockchain.getContractState(accessControlContract));
    assertThat(state.currentlyHeldBy().data()).isEqualTo(account2);

    assertThatThrownBy(() -> blockchain.sendAction(account4, accessControlContract, payload))
        .hasMessageContaining("Object is already lent out");
  }
}
