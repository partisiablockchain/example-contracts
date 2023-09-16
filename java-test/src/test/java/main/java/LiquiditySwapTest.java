package main.java;

import static org.assertj.core.api.Assertions.assertThat;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.LiquiditySwap;
import com.partisiablockchain.language.abicodegen.TokenContract;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import java.math.BigInteger;
import java.nio.file.Path;
import java.util.List;

/** {@link LiquiditySwap} testing. */
public final class LiquiditySwapTest extends LiquiditySwapBaseTest {

  /** {@link LiquiditySwap} contract bytes. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/liquidity_swap.wasm"),
          Path.of("../target/wasm32-unknown-unknown/release/liquidity_swap.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/liquidity_swap_runner"));

  private static final BigInteger TOTAL_SUPPLY_A = BigInteger.ONE.shiftLeft(60);
  private static final BigInteger TOTAL_SUPPLY_B = BigInteger.ONE.shiftLeft(62);
  private static final BigInteger AMOUNT_FOR_INITIAL_LIQUIDITY = BigInteger.ONE.shiftLeft(50);

  private BlockchainAddress creatorAddress;
  private BlockchainAddress contractSwap;

  private BlockchainAddress contractTokenA;
  private BlockchainAddress contractTokenB;

  @Override
  protected BlockchainAddress creatorAddress() {
    return creatorAddress;
  }

  @Override
  protected BlockchainAddress swapContractAddress() {
    return contractSwap;
  }

  @ContractTest
  void contractInit() {
    creatorAddress = blockchain.newAccount(1);
    // Setup tokens
    final byte[] initRpcA = TokenContract.initialize("Token A", "A", (byte) 8, TOTAL_SUPPLY_A);
    contractTokenA = blockchain.deployContract(creatorAddress, TokenTest.CONTRACT_BYTES, initRpcA);

    final byte[] initRpcB = TokenContract.initialize("Token B", "B", (byte) 8, TOTAL_SUPPLY_B);
    contractTokenB = blockchain.deployContract(creatorAddress, TokenTest.CONTRACT_BYTES, initRpcB);

    // Setup swap
    final byte[] initRpcSwap = LiquiditySwap.initialize(contractTokenA, contractTokenB, (short) 0);
    contractSwap = blockchain.deployContract(creatorAddress, CONTRACT_BYTES, initRpcSwap);

    // Deposit setup
    depositAmount(List.of(creatorAddress), contractTokenA, AMOUNT_FOR_INITIAL_LIQUIDITY);
    depositAmount(List.of(creatorAddress), contractTokenB, AMOUNT_FOR_INITIAL_LIQUIDITY);

    blockchain.sendAction(
        creatorAddress,
        contractSwap,
        LiquiditySwap.provideInitialLiquidity(
            AMOUNT_FOR_INITIAL_LIQUIDITY, AMOUNT_FOR_INITIAL_LIQUIDITY));
  }

  @ContractTest(previous = "contractInit")
  void contractInitCheck() {
    final LiquiditySwap.LiquiditySwapContractState state =
        LiquiditySwap.LiquiditySwapContractState.deserialize(
            blockchain.getContractState(contractSwap));

    assertThat(swapDepositBalance(state, creatorAddress, contractTokenA))
        .isEqualTo(BigInteger.ZERO);
    assertThat(swapDepositBalance(state, creatorAddress, contractTokenB))
        .isEqualTo(BigInteger.ZERO);
    assertThat(swapDepositBalance(state, contractSwap, contractTokenA))
        .isEqualTo(AMOUNT_FOR_INITIAL_LIQUIDITY);
    assertThat(swapDepositBalance(state, contractSwap, contractTokenB))
        .isEqualTo(AMOUNT_FOR_INITIAL_LIQUIDITY);
    assertThat(swapDepositBalance(state, contractSwap, contractSwap))
        .isEqualTo(AMOUNT_FOR_INITIAL_LIQUIDITY); // sqrt(i * i) = i
  }
}
