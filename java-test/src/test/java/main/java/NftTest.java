package main.java;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.NftContract;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import java.math.BigInteger;
import java.nio.file.Path;

/** NFT contract test. */
public final class NftTest extends JunitContractTest {
  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/nft_contract.wasm"),
          Path.of("../target/wasm32-unknown-unknown/release/nft_contract.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/nft_contract_runner"));

  private BlockchainAddress nftContractOwnerAccount;

  private BlockchainAddress token1Owner;
  private BlockchainAddress token2Owner;
  private BlockchainAddress token3Owner;
  private BlockchainAddress contractAddress;

  private static final BigInteger token1 = BigInteger.valueOf(1);
  private static final BigInteger token2 = BigInteger.valueOf(2);
  private static final BigInteger token3 = BigInteger.valueOf(3);
  private static final BigInteger token4 = BigInteger.valueOf(4);

  /** Test of initializer. */
  @ContractTest
  void init() {
    nftContractOwnerAccount = blockchain.newAccount(2);
    token1Owner = blockchain.newAccount(5);
    token2Owner = blockchain.newAccount(3);
    token3Owner = blockchain.newAccount(4);

    byte[] initRpc = NftContract.initialize("SomeNFT", "SNFT", "www.snft/");
    contractAddress = blockchain.deployContract(nftContractOwnerAccount, CONTRACT_BYTES, initRpc);

    NftContract.NFTContractState state = getState();

    assertThat(state.name()).isEqualTo("SomeNFT");
    assertThat(state.symbol()).isEqualTo("SNFT");
    assertThat(state.owners()).isEmpty();
    assertThat(state.tokenApprovals()).isEmpty();
    assertThat(state.operatorApprovals()).isEmpty();
    assertThat(state.uriTemplate()).isEqualTo("www.snft/");
    assertThat(state.tokenUriDetails()).isEmpty();
    assertThat(state.contractOwner()).isEqualTo(nftContractOwnerAccount);
  }

  /** The contract owner can mint tokens to accounts. */
  @ContractTest(previous = "init")
  void mint() {
    // The nft contract owner account mints tokens to other accounts
    byte[] mint1 = NftContract.mint(token1Owner, token1, uriFromByte(0x01));
    byte[] mint2 = NftContract.mint(token2Owner, token2, uriFromByte(0x02));
    byte[] mint3 = NftContract.mint(token3Owner, token3, uriFromByte(0x03));

    blockchain.sendAction(nftContractOwnerAccount, contractAddress, mint1);
    blockchain.sendAction(nftContractOwnerAccount, contractAddress, mint2);
    blockchain.sendAction(nftContractOwnerAccount, contractAddress, mint3);

    // Assert that the owners are correct
    NftContract.NFTContractState state = getState();
    assertThat(state.owners()).hasSize(3);
    assertThat(state.owners().get(token1)).isEqualTo(token1Owner);
    assertThat(state.owners().get(token2)).isEqualTo(token2Owner);
    assertThat(state.owners().get(token3)).isEqualTo(token3Owner);

    // Assert that the token uris are correct
    assertThat(state.tokenUriDetails()).hasSize(3);
    assertThat(state.tokenUriDetails().get(token1))
        .isEqualTo(new byte[] {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1});
    assertThat(state.tokenUriDetails().get(token2))
        .isEqualTo(new byte[] {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2});
    assertThat(state.tokenUriDetails().get(token3))
        .isEqualTo(new byte[] {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3});
  }

  /** An account which is not the nft contract owner cannot mint a token (not yet minted). */
  @ContractTest(previous = "init")
  void nonContractOwnerCannotMintNonMintedToken() {
    // Check that the account is not the Nft contract owner
    NftContract.NFTContractState state = getState();
    assertThat(state.contractOwner()).isNotEqualTo(token2Owner);
    // An account which is not the Nft contract owner tries to mint a non-minted token
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token2Owner,
                    contractAddress,
                    NftContract.mint(token2Owner, token4, uriFromByte(0x04))))
        .hasMessageContaining("MPC-721: mint only callable by the contract owner");
  }

  /** An account which is not the nft contract owner cannot mint a token (already minted). */
  @ContractTest(previous = "mint")
  void nonContractOwnerCannotMintMintedToken() {
    // Check that the account is not the Nft contract owner
    NftContract.NFTContractState state = getState();
    assertThat(state.contractOwner()).isNotEqualTo(token2Owner);
    // An account who is not the Nft contract owner tries to mint a minted token
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token2Owner,
                    contractAddress,
                    NftContract.mint(token2Owner, token1, uriFromByte(0x01))))
        .hasMessageContaining("MPC-721: mint only callable by the contract owner");
  }

  /** The same token cannot be minted more than once. */
  @ContractTest(previous = "mint")
  void cannotMintSameTokenTwice() {
    // Check that the token has been minted
    NftContract.NFTContractState state = getState();
    assertThat(state.owners().get(token1)).isNotNull();

    // Try to mint the token again
    byte[] mint1 = NftContract.mint(nftContractOwnerAccount, token1, uriFromByte(0x01));
    assertThatThrownBy(() -> blockchain.sendAction(nftContractOwnerAccount, contractAddress, mint1))
        .hasMessageContaining("MPC-721: token already minted");
  }

  /** Owner of a token can transfer its token to another account. */
  @ContractTest(previous = "mint")
  void ownerCanTransferFrom() {
    NftContract.NFTContractState state = getState();
    assertThat(state.owners().get(token3)).isEqualTo(token3Owner);

    blockchain.sendAction(
        token3Owner, contractAddress, NftContract.transferFrom(token3Owner, token2Owner, token3));
    assertThat(getState().owners().get(token3)).isEqualTo(token2Owner);
  }

  /** An account cannot transfer a non-existing token. */
  @ContractTest(previous = "init")
  void cannotTransferNonExistingToken() {
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token3Owner,
                    contractAddress,
                    NftContract.transferFrom(token3Owner, token2Owner, BigInteger.valueOf(4))))
        .hasMessageContaining("MPC-721: owner query for nonexistent token");
  }

  /** A token owner cannot transfer its token from another account. */
  @ContractTest(previous = "mint")
  void tokenOwnerCannotTransferFromAnotherAccount() {
    NftContract.NFTContractState state = getState();
    assertThat(state.owners().get(token1)).isEqualTo(token1Owner);

    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token1Owner,
                    contractAddress,
                    NftContract.transferFrom(token3Owner, token2Owner, token1)))
        .hasMessageContaining("MPC-721: transfer from incorrect owner");
  }

  /** The owner of a token can burn the token. */
  @ContractTest(previous = "mint")
  void burn() {
    assertThat(getState().owners().get(token2)).isEqualTo(token2Owner);

    blockchain.sendAction(token2Owner, contractAddress, NftContract.burn(token2));
    assertThat(getState().owners().get(token2)).isNull();
    assertThat(getState().tokenUriDetails().get(token2)).isNull();
  }

  /** A token which does not exist cannot be burnt. */
  @ContractTest(previous = "init")
  void cannotBurnNonExistingToken() {
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token2Owner, contractAddress, NftContract.burn(BigInteger.valueOf(4))))
        .hasMessageContaining("MPC-721: owner query for nonexistent token");
  }

  /** The owner of a token can approve another account for the token. */
  @ContractTest(previous = "mint")
  void ownerCanApprove() {
    assertThat(getState().owners().get(token2)).isEqualTo(token2Owner);

    blockchain.sendAction(token2Owner, contractAddress, NftContract.approve(token3Owner, token2));
    assertThat(getState().tokenApprovals()).hasSize(1);
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token3Owner);
  }

  /** The token owner can change which account is approved for the token. */
  @ContractTest(previous = "mint")
  void canChangeApproval() {
    assertThat(getState().owners().get(token2)).isEqualTo(token2Owner);
    // Token owner approves one account for a token, and then changes the approval to another
    // account
    blockchain.sendAction(token2Owner, contractAddress, NftContract.approve(token3Owner, token2));
    blockchain.sendAction(token2Owner, contractAddress, NftContract.approve(token1Owner, token2));
    assertThat(getState().tokenApprovals()).hasSize(1);
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token1Owner);
  }

  /** The token owner can remove an approval. */
  @ContractTest(previous = "ownerCanApprove")
  void canRemoveApproval() {
    // From the previous test, token3Owner is approved for token2
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token3Owner);
    blockchain.sendAction(token2Owner, contractAddress, NftContract.approve(null, token2));
    assertThat(getState().tokenApprovals()).isEmpty();
  }

  /** A non-existing token cannot be approved. */
  @ContractTest(previous = "mint")
  void cannotApproveNonExistingToken() {
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token2Owner,
                    contractAddress,
                    NftContract.approve(token3Owner, BigInteger.valueOf(4))))
        .hasMessageContaining("MPC-721: owner query for nonexistent token");
  }

  /** A caller who is neither owner nor operator of the token cannot approve the token. */
  @ContractTest(previous = "mint")
  void cannotApproveIfUnauthorized() {
    NftContract.NFTContractState state = getState();
    assertThat(state.owners().get(token1)).isNotEqualTo(token3Owner);
    assertThat(getState().tokenApprovals().get(token1)).isNotEqualTo(token3Owner);

    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token3Owner, contractAddress, NftContract.approve(token2Owner, token1)))
        .hasMessageContaining("MPC-721: approve caller is not owner nor authorized operator");
  }

  /** An account which has been approved for a token (i.e. operator) can transfer the token. */
  @ContractTest(previous = "ownerCanApprove")
  void approvedAccountTransfer() {
    // From the previous test, token3Owner is approved for token2
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token3Owner);
    blockchain.sendAction(
        token3Owner, contractAddress, NftContract.transferFrom(token2Owner, token1Owner, token2));
    assertThat(getState().owners().get(token2)).isEqualTo(token1Owner);
    // assert that approval of token is cleared
    assertThat(getState().tokenApprovals().get(token2)).isNull();
  }

  /** A token can be approved for the same account more than once. */
  @ContractTest(previous = "ownerCanApprove")
  void tokenCanBeApprovedAgain() {
    // Check that a token is approved from previous test
    assertThat(getState().tokenApprovals()).hasSize(1);
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token3Owner);
    // Approve the token again
    blockchain.sendAction(token2Owner, contractAddress, NftContract.approve(token3Owner, token2));
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token3Owner);
  }

  /** An account which has been approved for a token (i.e. operator) can burn the token. */
  @ContractTest(previous = "ownerCanApprove")
  void approvedAccountCanBurnToken() {
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token3Owner);

    blockchain.sendAction(token3Owner, contractAddress, NftContract.burn(token2));
    assertThat(getState().owners().get(token2)).isNull();
  }

  /** An account can approve another account for all of its assets. */
  @ContractTest(previous = "mint")
  void setApprovalForAll() {
    byte[] setApproval = NftContract.setApprovalForAll(token3Owner, true);
    blockchain.sendAction(token2Owner, contractAddress, setApproval);
    assertThat(getState().operatorApprovals())
        .containsExactly(new NftContract.OperatorApproval(token2Owner, token3Owner));
  }

  /** An account can set approval_for_all for more than one account. */
  @ContractTest(previous = "setApprovalForAll")
  void canSetAnotherApprovalForSameAccount() {
    // check the existing approval_for_all
    assertThat(getState().operatorApprovals())
        .containsExactly(new NftContract.OperatorApproval(token2Owner, token3Owner));

    blockchain.sendAction(
        token2Owner, contractAddress, NftContract.setApprovalForAll(token1Owner, true));
    assertThat(getState().operatorApprovals())
        .containsExactlyInAnyOrder(
            new NftContract.OperatorApproval(token2Owner, token3Owner),
            new NftContract.OperatorApproval(token2Owner, token1Owner));
  }

  /** An account can remove approval_for_all. */
  @ContractTest(previous = "setApprovalForAll")
  void canRemoveAnApproval() {
    // check the existing approval_for_all
    assertThat(getState().operatorApprovals())
        .containsExactly(new NftContract.OperatorApproval(token2Owner, token3Owner));

    blockchain.sendAction(
        token2Owner, contractAddress, NftContract.setApprovalForAll(token3Owner, false));
    assertThat(getState().operatorApprovals()).isEmpty();
  }

  /** An already set approval can be set again. */
  @ContractTest(previous = "setApprovalForAll")
  void canSetAnAlreadyExistingApproval() {
    // check the existing approval_for_all
    assertThat(getState().operatorApprovals())
        .containsExactly(new NftContract.OperatorApproval(token2Owner, token3Owner));

    // set already existing approval_for_all again
    byte[] setApproval = NftContract.setApprovalForAll(token3Owner, true);
    blockchain.sendAction(token2Owner, contractAddress, setApproval);
    assertThat(getState().operatorApprovals())
        .containsExactly(new NftContract.OperatorApproval(token2Owner, token3Owner));
  }

  /** An account cannot set approval_for_all to itself. */
  @ContractTest(previous = "mint")
  void cannotSetApproveAllForSelf() {
    // try setting approval_for_all to self
    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token2Owner, contractAddress, NftContract.setApprovalForAll(token2Owner, true)))
        .hasMessageContaining("MPC-721: approve to caller");
  }

  /** An operator with approval for all assets can approve a token. */
  @ContractTest(previous = "setApprovalForAll")
  void operatorCanApproveOwnersToken() {
    // check that account is operator
    assertThat(getState().operatorApprovals())
        .contains(new NftContract.OperatorApproval(token2Owner, token3Owner));
    // operator can approve owner's token
    blockchain.sendAction(token3Owner, contractAddress, NftContract.approve(token1Owner, token2));
    assertThat(getState().tokenApprovals().get(token2)).isEqualTo(token1Owner);
  }

  /** An operator with approval for all assets can burn a token. */
  @ContractTest(previous = "setApprovalForAll")
  void operatorCanBurnOwnersToken() {
    // check that account is operator
    assertThat(getState().operatorApprovals())
        .contains(new NftContract.OperatorApproval(token2Owner, token3Owner));
    blockchain.sendAction(token3Owner, contractAddress, NftContract.burn(token2));
    assertThat(getState().owners().get(token2)).isNull();
  }

  /** An operator with approval for all assets can transfer a token. */
  @ContractTest(previous = "setApprovalForAll")
  void operatorCanTransferOwnersToken() {
    // check that account is operator
    assertThat(getState().operatorApprovals())
        .contains(new NftContract.OperatorApproval(token2Owner, token3Owner));
    // operator can transfer owner's token
    blockchain.sendAction(
        token3Owner, contractAddress, NftContract.transferFrom(token2Owner, token1Owner, token2));
    assertThat(getState().owners().get(token2)).isEqualTo(token1Owner);
  }

  /** Accounts who do not own a token and are not approved cannot transfer the token. */
  @ContractTest(previous = "mint")
  void onlyOwnerOrOperatorCanTransferFrom() {
    // Check that account is neither operator nor owner of the token
    assertThat(getState().owners().get(token1)).isNotEqualTo(token2Owner);
    assertThat(getState().tokenApprovals().get(token1)).isNotEqualTo(token2Owner);

    assertThatThrownBy(
            () ->
                blockchain.sendAction(
                    token3Owner,
                    contractAddress,
                    NftContract.transferFrom(token3Owner, token2Owner, token1)))
        .hasMessageContaining("MPC-721: transfer caller is not owner nor approved");
  }

  /** Accounts who do not own a token and are not approved cannot burn the token. */
  @ContractTest(previous = "mint")
  void onlyOwnerOrOperatorCanBurn() {
    // Check that account is neither operator nor owner of the token
    assertThat(getState().owners().get(token1)).isNotEqualTo(token2Owner);
    assertThat(getState().tokenApprovals().get(token1)).isNotEqualTo(token2Owner);

    assertThatThrownBy(
            () -> blockchain.sendAction(token2Owner, contractAddress, NftContract.burn(token1)))
        .hasMessageContaining("MPC-721: burn caller is not owner nor approved");
  }

  private byte[] uriFromByte(int lastByte) {
    byte[] result = new byte[16];
    result[15] = (byte) lastByte;
    return result;
  }

  private NftContract.NFTContractState getState() {
    return NftContract.NFTContractState.deserialize(blockchain.getContractState(contractAddress));
  }
}
