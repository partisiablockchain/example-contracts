package main.java;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.LiquiditySwap;
import com.partisiablockchain.language.abicodegen.TokenContract;
import com.partisiablockchain.language.junit.ContractTest;
import java.math.BigInteger;
import java.util.List;
import org.assertj.core.api.Assertions;

/** {@link LiquiditySwap} testing. */
public final class LiquiditySwapEthUsdcTest extends LiquiditySwapBaseTest {

  private static final int DECIMALS_ETH = 18;
  private static final int DECIMALS_USDC = 3;

  private static final BigInteger BASE_UNIT_ETH = BigInteger.TEN.pow(DECIMALS_ETH);
  private static final BigInteger BASE_UNIT_USDC = BigInteger.TEN.pow(DECIMALS_USDC);

  private static final BigInteger TOTAL_SUPPLY_ETH =
      BigInteger.valueOf(120_000_000L).multiply(BASE_UNIT_ETH);
  private static final BigInteger TOTAL_SUPPLY_USDC =
      BigInteger.valueOf(21_200_000_000_000L).multiply(BASE_UNIT_USDC);

  private static final BigInteger INITIAL_NUM_ETH = BigInteger.valueOf(100);
  private static final BigInteger INITIAL_RATE_ETH_USDC = BigInteger.valueOf(1846);

  private static final BigInteger INITIAL_LIQUIDITY_ETH = INITIAL_NUM_ETH.multiply(BASE_UNIT_ETH);
  private static final BigInteger INITIAL_LIQUIDITY_USDC =
      INITIAL_NUM_ETH.multiply(INITIAL_RATE_ETH_USDC).multiply(BASE_UNIT_USDC);

  private static final short SWAP_FEE_PER_MILLE = (short) 0;

  private BlockchainAddress creatorAddress;
  private BlockchainAddress account2;

  private BlockchainAddress contractEth;
  private BlockchainAddress contractUsdCoin;
  private BlockchainAddress contractSwap;

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
    account2 = blockchain.newAccount(2);

    // Setup tokens
    final byte[] initRpcEth =
        TokenContract.initialize("Ethereum Ether", "ETH", (byte) DECIMALS_ETH, TOTAL_SUPPLY_ETH);
    contractEth = blockchain.deployContract(creatorAddress, CONTRACT_BYTES_TOKEN, initRpcEth);

    final byte[] initRpcUsdCoin =
        TokenContract.initialize("USD Coin", "USDC", (byte) DECIMALS_USDC, TOTAL_SUPPLY_USDC);
    contractUsdCoin =
        blockchain.deployContract(creatorAddress, CONTRACT_BYTES_TOKEN, initRpcUsdCoin);

    // Setup swap
    final byte[] initRpcSwap =
        LiquiditySwap.initialize(contractUsdCoin, contractEth, SWAP_FEE_PER_MILLE);
    contractSwap = blockchain.deployContract(creatorAddress, CONTRACT_BYTES_SWAP, initRpcSwap);

    // Deposit setup
    depositAmount(List.of(creatorAddress), contractEth, INITIAL_LIQUIDITY_ETH);
    depositAmount(List.of(creatorAddress), contractUsdCoin, INITIAL_LIQUIDITY_USDC);

    // Provide initial liquidity
    blockchain.sendAction(
        creatorAddress,
        contractSwap,
        LiquiditySwap.provideInitialLiquidity(INITIAL_LIQUIDITY_USDC, INITIAL_LIQUIDITY_ETH));

    // Validate
    validateBalance(creatorAddress, contractEth, BigInteger.ZERO);
    validateBalance(creatorAddress, contractUsdCoin, BigInteger.ZERO);
    validateBalance(contractSwap, contractEth, INITIAL_LIQUIDITY_ETH);
    validateBalance(contractSwap, contractUsdCoin, INITIAL_LIQUIDITY_USDC);
    validateExchangeRate(DECIMALS_ETH - DECIMALS_USDC, INITIAL_RATE_ETH_USDC);
  }

  @ContractTest(previous = "contractInit")
  void swapForOneEth() {
    final BigInteger usdcAmount =
        BigInteger.ONE.multiply(INITIAL_RATE_ETH_USDC).multiply(BASE_UNIT_USDC);

    depositAmount(List.of(account2), contractUsdCoin, usdcAmount);
    validateExchangeRate(DECIMALS_ETH - DECIMALS_USDC, INITIAL_RATE_ETH_USDC);
    validateBalance(account2, contractEth, BigInteger.ZERO);
    validateBalance(account2, contractUsdCoin, usdcAmount);
    swap(account2, contractUsdCoin, usdcAmount);
    validateBalance(account2, contractUsdCoin, BigInteger.ZERO);
    validateExchangeRate(DECIMALS_ETH - DECIMALS_USDC, BigInteger.valueOf(1883));
  }

  @ContractTest(previous = "swapForOneEth")
  void swapBack() {
    final BigInteger etcAmount = swapDepositBalance(account2, contractEth);

    // Lose one to odd exchange rate:
    final BigInteger expectedOutput =
        BigInteger.ONE
            .multiply(INITIAL_RATE_ETH_USDC)
            .multiply(BASE_UNIT_USDC)
            .subtract(BigInteger.ONE);

    swap(account2, contractEth, etcAmount);
    validateExchangeRate(DECIMALS_ETH - DECIMALS_USDC, INITIAL_RATE_ETH_USDC);
    validateBalance(account2, contractEth, BigInteger.ZERO);
    validateBalance(account2, contractUsdCoin, expectedOutput);
  }

  @ContractTest(previous = "swapForOneEth")
  void swapBackButExpectTooMuch() {
    final BigInteger etcAmount = swapDepositBalance(account2, contractEth);
    // Lose one to odd exchange rate, add one to miscalculated output.
    final BigInteger minimumOutput =
        BigInteger.ONE.multiply(INITIAL_RATE_ETH_USDC).multiply(BASE_UNIT_USDC);

    Assertions.assertThatCode(() -> swap(account2, contractEth, etcAmount, minimumOutput))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining(
            "Swap produced 1845999 output tokens, but minimum was set to 1846000.");
  }
}
