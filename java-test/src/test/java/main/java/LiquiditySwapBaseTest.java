package main.java;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.LiquiditySwap;
import com.partisiablockchain.language.abicodegen.TokenContract;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.JunitContractTest;
import java.math.BigInteger;
import java.nio.file.Path;
import java.util.List;
import java.util.stream.Collectors;
import org.assertj.core.api.Assertions;

/** {@link LiquiditySwap} testing. */
public abstract class LiquiditySwapBaseTest extends JunitContractTest {

  protected static final ContractBytes CONTRACT_BYTES_TOKEN = TokenTest.CONTRACT_BYTES;

  protected static final ContractBytes CONTRACT_BYTES_SWAP =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/liquidity_swap.wasm"),
          Path.of("../target/wasm32-unknown-unknown/release/liquidity_swap.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/liquidity_swap_runner"));

  protected abstract BlockchainAddress creatorAddress();

  protected abstract BlockchainAddress swapContractAddress();

  /** State accessor for token balances. */
  protected static LiquiditySwap.TokenBalance getSwapDepositBalances(
      LiquiditySwap.LiquiditySwapContractState state, BlockchainAddress owner) {
    return state.tokenBalances().balances().get(owner);
  }

  /** State accessor for deposits on liquidity swap contracts. */
  private static BigInteger swapDepositBalance(
      LiquiditySwap.LiquiditySwapContractState state,
      LiquiditySwap.TokenBalance tokenBalance,
      BlockchainAddress tokenAddress) {
    if (tokenBalance == null) {
      return BigInteger.ZERO;
    }
    if (tokenAddress.equals(state.liquidityPoolAddress())) {
      return tokenBalance.liquidityTokens();
    }
    if (tokenAddress.equals(state.tokenBalances().tokenAAddress())) {
      return tokenBalance.aTokens();
    }
    if (tokenAddress.equals(state.tokenBalances().tokenBAddress())) {
      return tokenBalance.bTokens();
    }
    return null;
  }

  /**
   * State accessor for deposits on liquidity swap contracts.
   *
   * <p>Token addresses are determined from the contract state. Liquidity tokens can be queried with
   * the liquidity swap contract's address.
   *
   * @param state Liquidity swap state.
   * @param owner Owner of the deposit
   * @param tokenAddress Addess of the token to determine balance for.
   * @return Token balance.
   */
  public static BigInteger swapDepositBalance(
      final LiquiditySwap.LiquiditySwapContractState state,
      final BlockchainAddress owner,
      final BlockchainAddress tokenAddress) {
    return swapDepositBalance(state, state.tokenBalances().balances().get(owner), tokenAddress);
  }

  /** State accessor for token balances. */
  protected final BigInteger swapDepositBalance(
      BlockchainAddress owner, BlockchainAddress tokenAddr) {
    final LiquiditySwap.LiquiditySwapContractState state =
        LiquiditySwap.LiquiditySwapContractState.deserialize(
            blockchain.getContractState(swapContractAddress()));
    return swapDepositBalance(state, owner, tokenAddr);
  }

  /** State validation. */
  protected final void validateStateInvariants(
      final LiquiditySwap.LiquiditySwapContractState state) {
    // Check that no accounts are empty
    for (final LiquiditySwap.TokenBalance tokenBalance :
        state.tokenBalances().balances().values()) {
      final List<BigInteger> hasAnyTokens =
          List.of(tokenBalance.aTokens(), tokenBalance.bTokens(), tokenBalance.liquidityTokens());
      Assertions.assertThat(hasAnyTokens)
          .as("TokenBalance must contain at least one non-zero field")
          .anyMatch(n -> !BigInteger.ZERO.equals(n));
    }

    // Check that liquidity tokens are correctly tracked.
    final BigInteger allLiquidityTokensIncludingBuiltInSum =
        state.tokenBalances().balances().values().stream()
            .map(LiquiditySwap.TokenBalance::liquidityTokens)
            .collect(Collectors.reducing(BigInteger.ZERO, BigInteger::add));
    final BigInteger liquidityTokenBuiltInSum =
        swapDepositBalance(state, state.liquidityPoolAddress(), state.liquidityPoolAddress());

    Assertions.assertThat(liquidityTokenBuiltInSum)
        .as("Liquidity-token built-in sum must be equal to all other liquidity token balances")
        .isEqualTo(allLiquidityTokensIncludingBuiltInSum.subtract(liquidityTokenBuiltInSum));

    // Check that initialized pools are consistent.
    final LiquiditySwap.TokenBalance tokenBalance =
        getSwapDepositBalances(state, state.liquidityPoolAddress());
    final List<BigInteger> hasAnyTokens =
        List.of(tokenBalance.aTokens(), tokenBalance.bTokens(), tokenBalance.liquidityTokens());
    final boolean expectedZeroes = hasAnyTokens.contains(BigInteger.ZERO);
    if (expectedZeroes) {
      Assertions.assertThat(hasAnyTokens)
          .containsExactly(BigInteger.ZERO, BigInteger.ZERO, BigInteger.ZERO);
    } else {
      Assertions.assertThat(hasAnyTokens).doesNotContain(BigInteger.ZERO);
    }
  }

  /** State validation. */
  protected final void validateStateInvariants() {
    final LiquiditySwap.LiquiditySwapContractState state =
        LiquiditySwap.LiquiditySwapContractState.deserialize(
            blockchain.getContractState(swapContractAddress()));
    validateStateInvariants(state);
  }

  /** State access. */
  protected final BigInteger exchangeRate(int precision) {
    final LiquiditySwap.LiquiditySwapContractState state =
        LiquiditySwap.LiquiditySwapContractState.deserialize(
            blockchain.getContractState(swapContractAddress()));
    final var aTokens =
        swapDepositBalance(
            state, state.liquidityPoolAddress(), state.tokenBalances().tokenAAddress());
    final var bTokens =
        swapDepositBalance(
            state, state.liquidityPoolAddress(), state.tokenBalances().tokenBAddress());
    return BigInteger.TEN.pow(precision).multiply(aTokens).divide(bTokens);
  }

  /**
   * State modifier for depositing, with automatic transfer from the token {@link creatorAddress}.
   */
  protected final void depositAmount(
      List<BlockchainAddress> senders, BlockchainAddress contractToken, BigInteger amount) {
    final var transfers = senders.stream().map(s -> new TokenContract.Transfer(s, amount)).toList();
    blockchain.sendAction(creatorAddress(), contractToken, TokenContract.bulkTransfer(transfers));

    for (final BlockchainAddress sender : senders) {
      blockchain.sendAction(
          sender, contractToken, TokenContract.approve(swapContractAddress(), amount));
      blockchain.sendAction(
          sender, swapContractAddress(), LiquiditySwap.deposit(contractToken, amount));
    }
  }

  protected final void swap(
      BlockchainAddress swapper, BlockchainAddress tokenInput, BigInteger amountInput) {
    swap(swapper, tokenInput, amountInput, BigInteger.ONE);
  }

  protected final void swap(
      BlockchainAddress swapper,
      BlockchainAddress tokenInput,
      BigInteger amountInput,
      BigInteger amountOutputMinimum) {
    final byte[] rpc = LiquiditySwap.swap(tokenInput, amountInput, amountOutputMinimum);
    blockchain.sendAction(swapper, swapContractAddress(), rpc);
  }

  protected final void validateExchangeRate(int precision, BigInteger expectedExchangeRate) {
    Assertions.assertThat(exchangeRate(precision))
        .as("Exchange rate")
        .isEqualTo(expectedExchangeRate);
  }

  protected final void validateBalance(
      BlockchainAddress swapper, BlockchainAddress tokenInput, BigInteger amountInput) {
    Assertions.assertThat(swapDepositBalance(swapper, tokenInput))
        .as("Balance of user: " + swapper)
        .isEqualTo(amountInput);
  }
}
