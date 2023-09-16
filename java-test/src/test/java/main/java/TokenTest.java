package main.java;

import static org.assertj.core.api.Assertions.assertThat;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.TokenContract;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.TestBlockchain;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import java.math.BigInteger;
import java.nio.file.Path;
import java.util.List;
import org.assertj.core.api.Assertions;

/** Test the Token contract. */
public final class TokenTest extends JunitContractTest {

  /** {@link ContractBytes} for {@link TokenContract}. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/token_contract.wasm"),
          Path.of("../target/wasm32-unknown-unknown/release/token_contract.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/token_contract_runner"));

  private static final BigInteger TOTAL_SUPPLY = BigInteger.valueOf(123123);

  private BlockchainAddress account1;
  private BlockchainAddress account2;
  private BlockchainAddress account3;
  private BlockchainAddress account4;
  private BlockchainAddress token;

  /** Setup for all the other tests. Deploys token contract and instantiates accounts. */
  @ContractTest
  void setup() {
    account1 = blockchain.newAccount(2);
    account2 = blockchain.newAccount(3);
    account3 = blockchain.newAccount(4);
    account4 = blockchain.newAccount(5);

    token =
        deployTokenContract(blockchain, account1, "My Cool Token", "COOL", (byte) 8, TOTAL_SUPPLY);

    final BigInteger transferAmount = BigInteger.ONE;

    transfer(account1, account2, transferAmount);

    final TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertStateInvariants(state);
    assertThat(state.totalSupply()).isEqualTo(TOTAL_SUPPLY);
    assertThat(state.balances().get(account1)).isEqualTo(TOTAL_SUPPLY.subtract(transferAmount));
    assertThat(state.balances().get(account2)).isEqualTo(transferAmount);
  }

  /** Transfer ten tokens to another account. */
  @ContractTest(previous = "setup")
  void transferTenTokens() {

    TokenContract.TokenState tokenState =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    BigInteger balanceBeforeTransfer = tokenState.balances().get(account1);

    transfer(account1, account3, BigInteger.TEN);

    tokenState = TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertThat(tokenState.balances().get(account1))
        .isEqualTo(balanceBeforeTransfer.subtract(BigInteger.TEN));
    assertStateInvariants(tokenState);
    assertThat(tokenState.balances().get(account3)).isEqualTo(BigInteger.TEN);
  }

  /** A transfer that makes the token amount go to zero, removes the account from balances. */
  @ContractTest(previous = "setup")
  void removeAccountWhenBalanceIsZero() {
    TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account3)).isNull();

    transfer(account2, account3, BigInteger.ONE);
    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertStateInvariants(state);
    assertThat(state.balances().get(account3)).isEqualTo(BigInteger.ONE);
    assertThat(state.balances().get(account2)).isNull();
  }

  /** Transfer from another account with transfer_from, from an approved account. */
  @ContractTest(previous = "setup")
  void transferApprovedTokens() {
    TokenContract.TokenState state;
    assertThat(allowance(account1, account2)).isNull();

    approve(account1, account2, BigInteger.valueOf(100));

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertStateInvariants(state);
    assertThat(allowance(account1, account2)).isEqualTo(BigInteger.valueOf(100));

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(allowance(account1, account2)).isEqualTo(BigInteger.valueOf(100));

    transferFrom(account2, account1, account3, BigInteger.TEN);

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertStateInvariants(state);
    assertThat(allowance(account1, account2)).isEqualTo(BigInteger.valueOf(90));
    assertThat(state.balances().get(account3)).isEqualTo(BigInteger.TEN);
  }

  /** Bulk transfer from one account to two other accounts. */
  @ContractTest(previous = "setup")
  void bulkTransferTokens() {
    TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account3)).isNull();
    byte[] transfer = TokenContract.transfer(account2, BigInteger.TWO);
    blockchain.sendAction(account1, token, transfer);

    List<TokenContract.Transfer> transfers =
        List.of(
            new TokenContract.Transfer(account3, BigInteger.ONE),
            new TokenContract.Transfer(account4, BigInteger.ONE));

    byte[] bulkTransfer = TokenContract.bulkTransfer(transfers);
    blockchain.sendAction(account2, token, bulkTransfer);
    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account3)).isEqualTo(BigInteger.ONE);
    assertThat(state.balances().get(account4)).isEqualTo(BigInteger.ONE);
    assertThat(state.balances().get(account2)).isEqualTo(BigInteger.ONE);
  }

  /** Bulk transfer for another account with approved account to two other accounts. */
  @ContractTest(previous = "setup")
  void bulkTransferFrom() {
    TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account3)).isNull();

    byte[] approve = TokenContract.approve(account4, BigInteger.valueOf(2));
    blockchain.sendAction(account1, token, approve);

    List<TokenContract.Transfer> transfers =
        List.of(
            new TokenContract.Transfer(account2, BigInteger.ONE),
            new TokenContract.Transfer(account3, BigInteger.ONE));

    byte[] bulkTransfer = TokenContract.bulkTransferFrom(account1, transfers);
    blockchain.sendAction(account4, token, bulkTransfer);

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account2)).isEqualTo(BigInteger.TWO);
    assertThat(state.balances().get(account3)).isEqualTo(BigInteger.ONE);
  }

  /** Transfer zero tokens to an account have no effect. */
  @ContractTest(previous = "setup")
  void transferZeroTokens() {

    TokenContract.TokenState tokenState =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    BigInteger balanceBeforeAttempt = tokenState.balances().get(account1);

    transfer(account1, account3, BigInteger.ZERO);

    tokenState = TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertThat(tokenState.balances().get(account1)).isEqualTo(balanceBeforeAttempt);
    assertStateInvariants(tokenState);
  }

  /** Transfer from account not present if the balance map fails. */
  @ContractTest(previous = "setup")
  void underflowTransferBalanceNonExistent() {
    TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account3)).isNull();
    BigInteger account1Balance = state.balances().get(account1);

    byte[] transfer = TokenContract.transfer(account1, BigInteger.ONE);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(account3, token, transfer))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Insufficient funds for transfer: 0/1");

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account1)).isEqualTo(account1Balance);
  }

  /** Transfer more tokens than account currently holds fails. */
  @ContractTest(previous = "setup")
  void underflowTransferInsufficientFunds() {
    byte[] transferToAccount3 = TokenContract.transfer(account3, BigInteger.ONE);
    blockchain.sendAction(account1, token, transferToAccount3);

    TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    BigInteger account1Balance = state.balances().get(account1);

    byte[] transfer = TokenContract.transfer(account1, BigInteger.TWO);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(account3, token, transfer))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Insufficient funds for transfer: 1/2");

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account1)).isEqualTo(account1Balance);
    assertThat(state.balances().get(account3)).isEqualTo(BigInteger.ONE);
  }

  /** Transfer for another account, without being approved for transfer fails. */
  @ContractTest(previous = "setup")
  void transferFromNonApprovedTokens() {

    TokenContract.TokenState state;

    Assertions.assertThatThrownBy(() -> transferFrom(account3, account1, account2, BigInteger.TEN))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Insufficient allowance for transfer_from: 0/10");

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertStateInvariants(state);
    assertThat(allowance(account1, account2)).isNull();
  }

  /**
   * Bulk transfer from one account to two other accounts - owner does not have enough tokens for
   * both - fails.
   */
  @ContractTest(previous = "setup")
  void bulkTransferTokensOneFails() {
    TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertThat(state.balances().get(account3)).isNull();

    assertThat(state.balances().get(account2)).isEqualTo(BigInteger.ONE);

    List<TokenContract.Transfer> transfers =
        List.of(
            new TokenContract.Transfer(account3, BigInteger.ONE),
            new TokenContract.Transfer(account4, BigInteger.ONE));

    byte[] bulkTransfer = TokenContract.bulkTransfer(transfers);

    Assertions.assertThatThrownBy(() -> blockchain.sendAction(account2, token, bulkTransfer))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Insufficient funds for transfer: 0/1");

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    assertStateInvariants(state);
    assertThat(state.balances().get(account2)).isEqualTo(BigInteger.ONE);
    assertThat(state.balances().get(account3)).isNull();
    assertThat(state.balances().get(account4)).isNull();
  }

  /** Bulk transfer of an empty list of transfers has no effect. */
  @ContractTest(previous = "setup")
  void bulkTransferNoTransfers() {
    TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertThat(state.balances().get(account2)).isEqualTo(BigInteger.ONE);

    List<TokenContract.Transfer> transfers = List.of();

    byte[] bulkTransfer = TokenContract.bulkTransfer(transfers);
    blockchain.sendAction(account2, token, bulkTransfer);

    state = TokenContract.TokenState.deserialize(blockchain.getContractState(token));

    assertStateInvariants(state);
    assertThat(state.balances().get(account2)).isEqualTo(BigInteger.ONE);
  }

  /** Helper function for making transfer RPC and invoking the transfer action. */
  private void transfer(BlockchainAddress from, BlockchainAddress to, BigInteger amount) {
    final byte[] rpc = TokenContract.transfer(to, amount);
    blockchain.sendAction(from, token, rpc);
  }

  /** Helper function for making approve RPC and invoking the approve action. */
  private void approve(BlockchainAddress approver, BlockchainAddress approvee, BigInteger amount) {
    final byte[] rpc = TokenContract.approve(approvee, amount);
    blockchain.sendAction(approver, token, rpc);
  }

  /** Helper function for making transferFrom RPC and invoking the transferFrom action. */
  private void transferFrom(
      BlockchainAddress approvee, BlockchainAddress from, BlockchainAddress to, BigInteger amount) {
    final byte[] rpc = TokenContract.transferFrom(from, to, amount);
    blockchain.sendAction(approvee, token, rpc);
  }

  /**
   * Deploy a Token contract with the given argument.
   *
   * @param blockchain the blockchain to deploy to.
   * @param creator the creator of the contract.
   * @param tokenName the token name.
   * @param tokenSymbol the token symbol.
   * @param decimals the amount of decimals a token can have.
   * @param totalSupply the total supply of a token.
   * @return the address for the deployed token contract
   */
  public static BlockchainAddress deployTokenContract(
      TestBlockchain blockchain,
      BlockchainAddress creator,
      String tokenName,
      String tokenSymbol,
      byte decimals,
      BigInteger totalSupply) {
    final byte[] initRpc = TokenContract.initialize(tokenName, tokenSymbol, decimals, totalSupply);
    final BlockchainAddress address = blockchain.deployContract(creator, CONTRACT_BYTES, initRpc);

    final TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(address));

    assertThat(state.name()).isEqualTo(tokenName);
    assertThat(state.symbol()).isEqualTo(tokenSymbol);
    assertThat(state.totalSupply()).isEqualTo(totalSupply);
    return address;
  }

  private BigInteger allowance(final BlockchainAddress owner, final BlockchainAddress spender) {
    final TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(token));
    final var ownerAllowances = state.allowed().get(owner);
    return ownerAllowances == null ? null : ownerAllowances.get(spender);
  }

  /**
   * Helper function for asserting the invariants that the balance map of the contract must not
   * contain zero-entries and that the sum of all the saved balances should equal the total supply.
   */
  private static void assertStateInvariants(TokenContract.TokenState state) {
    BigInteger allAssignedBalances = BigInteger.ZERO;

    for (final var balance : state.balances().values()) {
      assertThat(balance).as("State must not contain balances with zero tokens").isNotZero();
      allAssignedBalances = allAssignedBalances.add(balance);
    }

    assertThat(allAssignedBalances)
        .as("Number of assigned tokens must be identical to total supply")
        .isEqualTo(state.totalSupply());
  }
}
