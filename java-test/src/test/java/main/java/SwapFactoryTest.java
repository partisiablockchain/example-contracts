package main.java;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatCode;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.DexSwapFactory;
import com.partisiablockchain.language.abicodegen.LiquiditySwap;
import com.partisiablockchain.language.abicodegen.TokenContract;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import java.math.BigInteger;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;

/** {@link DexSwapFactory} contract testing. */
public final class SwapFactoryTest extends JunitContractTest {

  /** {@link DexSwapFactory} contract bytes. */
  public static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/dex_swap_factory.wasm"),
          Path.of("../target/wasm32-unknown-unknown/release/dex_swap_factory.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/dex_swap_factory_runner"));

  private static final BigInteger TOTAL_SUPPLY =
      BigInteger.valueOf(1200).multiply(BigInteger.TEN.pow(18));
  private static final BigInteger INITIAL_ACCOUNT_TOKENS =
      BigInteger.valueOf(100).multiply(BigInteger.TEN.pow(18));

  private BlockchainAddress creator;
  private BlockchainAddress liquidityProvider;
  private BlockchainAddress swapper;

  private BlockchainAddress token1;
  private BlockchainAddress token2;
  private BlockchainAddress swapFactory;
  private BlockchainAddress swapAddress;

  @ContractTest
  void setupTokenContracts() {
    // Setup accounts
    creator = blockchain.newAccount(99);
    liquidityProvider = blockchain.newAccount(2);
    swapper = blockchain.newAccount(3);

    // Setup tokens
    token1 =
        TokenTest.deployTokenContract(
            blockchain, creator, "Ethereum", "ETH", (byte) 18, TOTAL_SUPPLY);
    token2 =
        TokenTest.deployTokenContract(
            blockchain, creator, "USD Coin", "USDC", (byte) 18, TOTAL_SUPPLY);
    System.out.println("A: " + token1);
    System.out.println("B: " + token2);

    // Move some tokens to accounts
    transfer(token1, creator, liquidityProvider, INITIAL_ACCOUNT_TOKENS);
    transfer(token1, creator, swapper, INITIAL_ACCOUNT_TOKENS);
    transfer(token2, creator, liquidityProvider, INITIAL_ACCOUNT_TOKENS);
    transfer(token2, creator, swapper, INITIAL_ACCOUNT_TOKENS);
  }

  @ContractTest(previous = "setupTokenContracts")
  void setupFactory() {
    final DexSwapFactory.Permission permissionUpdateSwap =
        new DexSwapFactory.Permission.Specific(List.of(creator));
    final DexSwapFactory.Permission permissionDeploySwap =
        new DexSwapFactory.Permission.Specific(List.of(creator, liquidityProvider));
    final byte[] initRpc =
        DexSwapFactory.initialize(permissionUpdateSwap, permissionDeploySwap, (short) 3);
    swapFactory = blockchain.deployContract(creator, CONTRACT_BYTES, initRpc);

    // Assess state
    final DexSwapFactory.SwapFactoryState state =
        DexSwapFactory.SwapFactoryState.deserialize(blockchain.getContractState(swapFactory));
    assertThat(state.permissionUpdateSwap()).isEqualTo(permissionUpdateSwap);
    assertThat(state.permissionDeploySwap()).isEqualTo(permissionDeploySwap);
    assertThat(state.swapContracts()).isEmpty();
    assertThat(state.swapContractBinary()).isNull();
    assertThat(state.swapFeePerMille()).isEqualTo((short) 3);
  }

  @ContractTest(previous = "setupFactory")
  void failWhenCreatorDeploysBeforeSettingCode() {
    // Make invocation
    final DexSwapFactory.TokenPair tokenPair = tokenPair(token1, token2);

    final byte[] rpc = DexSwapFactory.deploySwapContract(tokenPair);
    assertThatCode(() -> blockchain.sendAction(creator, swapFactory, rpc))
        .hasMessageContaining(
            "Cannot deploy swap contract, when swap contract binary has not be set!");
  }

  @ContractTest(previous = "setupFactory")
  void setContractCode() {
    updateSwapBinary(
        creator,
        1_0_0,
        LiquiditySwapTest.CONTRACT_BYTES.code(),
        LiquiditySwapTest.CONTRACT_BYTES.abi());

    // Assess state
    final DexSwapFactory.SwapFactoryState state =
        DexSwapFactory.SwapFactoryState.deserialize(blockchain.getContractState(swapFactory));
    assertThat(state.swapContractBinary()).isNotNull();
    assertThat(state.swapContractBinary().version()).isEqualTo(1_0_0);
  }

  @ContractTest(previous = "setupFactory")
  void failWhenNonCreatorAttemptsToUpdateContract() {
    assertThatCode(
            () ->
                updateSwapBinary(
                    swapper,
                    1_0_0,
                    LiquiditySwapTest.CONTRACT_BYTES.code(),
                    LiquiditySwapTest.CONTRACT_BYTES.abi()))
        .hasMessageContaining("did not have permission \"update swap\"");
  }

  @ContractTest(previous = "setContractCode")
  void createFirstSwap() {
    // Make invocation
    final DexSwapFactory.TokenPair tokenPair = tokenPair(token1, token2);

    final byte[] rpc = DexSwapFactory.deploySwapContract(tokenPair);
    blockchain.sendAction(liquidityProvider, swapFactory, rpc);

    // Check state
    final DexSwapFactory.SwapFactoryState state =
        DexSwapFactory.SwapFactoryState.deserialize(blockchain.getContractState(swapFactory));
    assertThat(state.swapContracts()).hasSize(1);
    swapAddress = state.swapContracts().keySet().iterator().next();
    assertThat(swapAddress).isNotNull();
    final var info = state.swapContracts().get(swapAddress);
    assertThat(info).isNotNull();
    assertThat(info.contractVersion()).isEqualTo(1_0_0);
    assertThat(info.successfullyDeployed()).isTrue();
    assertThat(info.tokenPair()).isEqualTo(tokenPair);
  }

  @ContractTest(previous = "setContractCode")
  void failWhenNonCreatorAttemptsToDeployContract() {
    // Make invocation
    final DexSwapFactory.TokenPair tokenPair = tokenPair(token1, token2);

    final byte[] rpc = DexSwapFactory.deploySwapContract(tokenPair);
    assertThatCode(() -> blockchain.sendAction(swapper, swapFactory, rpc))
        .hasMessageContaining("did not have permission \"deploy swap\"");
  }

  @ContractTest(previous = "createFirstSwap")
  void setupLiquidityProvider() {

    // Deposit tokens
    approve(liquidityProvider, token1, swapAddress, INITIAL_ACCOUNT_TOKENS);
    deposit(liquidityProvider, token1, INITIAL_ACCOUNT_TOKENS);
    approve(liquidityProvider, token2, swapAddress, INITIAL_ACCOUNT_TOKENS);
    deposit(liquidityProvider, token2, INITIAL_ACCOUNT_TOKENS);

    assertThat(swapDepositBalance(liquidityProvider, token1)).isEqualTo(INITIAL_ACCOUNT_TOKENS);
    assertThat(swapDepositBalance(liquidityProvider, token2)).isEqualTo(INITIAL_ACCOUNT_TOKENS);
    assertThat(tokenBalance(liquidityProvider, token1)).isZero();
    assertThat(tokenBalance(liquidityProvider, token2)).isZero();

    // Now become liquidity provider
    provideInitialLiquidity(liquidityProvider, INITIAL_ACCOUNT_TOKENS, INITIAL_ACCOUNT_TOKENS);

    assertThat(swapDepositBalance(liquidityProvider, token1)).isZero();
    assertThat(swapDepositBalance(liquidityProvider, token2)).isZero();
    assertThat(swapDepositBalance(liquidityProvider, swapAddress)).isPositive();
  }

  @ContractTest(previous = "setupLiquidityProvider")
  void performSwap() {

    // Setup swap
    approve(swapper, token1, swapAddress, INITIAL_ACCOUNT_TOKENS);
    deposit(swapper, token1, INITIAL_ACCOUNT_TOKENS);

    assertThat(swapDepositBalance(swapper, token1)).isPositive();
    assertThat(swapDepositBalance(swapper, token2)).isZero();

    swap(swapper, token1, INITIAL_ACCOUNT_TOKENS);

    // Check state
    assertThat(swapDepositBalance(swapper, token1)).isZero();
    assertThat(swapDepositBalance(swapper, token2))
        .isPositive()
        .isEqualTo(new BigInteger("335160658106955"));
  }

  // TODO: Need check that swap_contract_exists_callback is called when
  // failing.

  @ContractTest(previous = "performSwap")
  void failWhenVersionsArentIncreasing() {
    assertThatCode(
            () ->
                updateSwapBinary(
                    creator,
                    1_0,
                    LiquiditySwapTest.CONTRACT_BYTES.code(),
                    LiquiditySwapTest.CONTRACT_BYTES.abi()))
        .hasMessageContaining(
            "Versions must be increasing: Previous version 100 should be less than new version 10");
  }

  @ContractTest(previous = "performSwap")
  void delistSwapContract() {
    final byte[] rpc = DexSwapFactory.delistSwapContract(swapAddress);
    blockchain.sendAction(creator, swapFactory, rpc);

    final DexSwapFactory.SwapFactoryState state =
        DexSwapFactory.SwapFactoryState.deserialize(blockchain.getContractState(swapFactory));
    assertThat(state.swapContracts()).isEmpty();
  }

  @ContractTest(previous = "delistSwapContract")
  void performSwapOnDelistedContract() {
    final BigInteger amountSwappable = swapDepositBalance(swapper, token2);
    assertThat(amountSwappable).isEqualTo(new BigInteger("335160658106955"));
    // .isBetween(INITIAL_ACCOUNT_TOKENS.shiftRight(1), INITIAL_ACCOUNT_TOKENS);

    swap(swapper, token2, amountSwappable);

    assertThat(swapDepositBalance(swapper, token2)).isZero();
  }

  @ContractTest(previous = "performSwapOnDelistedContract")
  void extractLiquidityFromDelistedContract() {

    // Stop being liquidity provider
    reclaimLiquidity(liquidityProvider, swapDepositBalance(liquidityProvider, swapAddress));

    // Withdraw from contract
    final BigInteger token1Withdrawable = swapDepositBalance(liquidityProvider, token1);
    final BigInteger token2Withdrawable = swapDepositBalance(liquidityProvider, token2);

    withdraw(liquidityProvider, token1, token1Withdrawable);
    withdraw(liquidityProvider, token2, token2Withdrawable);

    assertThat(swapDepositBalance(liquidityProvider, token1))
        .as("liquidityProvider deposit token1")
        .isZero()
        .isNotEqualTo(token1Withdrawable);
    assertThat(swapDepositBalance(liquidityProvider, token2))
        .as("liquidityProvider deposit token2")
        .isZero()
        .isNotEqualTo(token2Withdrawable);

    assertThat(tokenBalance(liquidityProvider, token1))
        .as("liquidityProvider token1 balance")
        .isEqualTo(token1Withdrawable)
        .isGreaterThan(BigInteger.ONE);
    assertThat(tokenBalance(liquidityProvider, token2))
        .as("liquidityProvider token2 balance")
        .isEqualTo(token2Withdrawable)
        .isGreaterThan(BigInteger.ONE);
  }

  @ContractTest(previous = "createFirstSwap")
  void createSecondSwapContract() {
    // Deploy second swap contract with same token pair
    final DexSwapFactory.TokenPair tokenPair = tokenPair(token1, token2);
    final byte[] rpc = DexSwapFactory.deploySwapContract(tokenPair);
    blockchain.sendAction(creator, swapFactory, rpc);

    // Check state
    final DexSwapFactory.SwapFactoryState state =
        DexSwapFactory.SwapFactoryState.deserialize(blockchain.getContractState(swapFactory));
    assertThat(state.swapContracts()).hasSize(2);
  }

  @ContractTest(previous = "createFirstSwap")
  void failWhenDeployingSelfSwapper() {
    // Deploy second swap contract with same token pair
    final DexSwapFactory.TokenPair tokenPair = tokenPair(token1, token1);
    final byte[] rpc = DexSwapFactory.deploySwapContract(tokenPair);
    assertThatCode(() -> blockchain.sendAction(creator, swapFactory, rpc))
        .hasMessageContaining("Tokens A and B must not be the same contract");

    // Check state
    final DexSwapFactory.SwapFactoryState state =
        DexSwapFactory.SwapFactoryState.deserialize(blockchain.getContractState(swapFactory));
    assertThat(state.swapContracts()).hasSize(1);

    assertThat(tokenPair(token1, token2)).isEqualTo(tokenPair(token2, token1));
  }

  @ContractTest(previous = "createSecondSwapContract")
  void failWhenUpdatingWithNonCode() {
    assertThatCode(
            () ->
                updateSwapBinary(
                    creator, 2_0_0, new byte[0], LiquiditySwapTest.CONTRACT_BYTES.abi()))
        .hasMessageContaining("Bytecode does not contain WASM code");
  }

  @ContractTest(previous = "createSecondSwapContract")
  void failWhenUpdatingWithNonAbi() {
    assertThatCode(
            () ->
                updateSwapBinary(
                    creator, 2_0_0, LiquiditySwapTest.CONTRACT_BYTES.code(), new byte[0]))
        .hasMessageContaining("ABI data invalid");
  }

  @ContractTest(previous = "setupTokenContracts")
  void setupFactoryWhereAnybodyCanCallDeploy() {
    // Deploy factory
    final DexSwapFactory.Permission permissionUpdateSwap =
        new DexSwapFactory.Permission.Specific(List.of(creator));
    final DexSwapFactory.Permission permissionDeploySwap = new DexSwapFactory.Permission.Anybody();

    final byte[] initRpc =
        DexSwapFactory.initialize(permissionUpdateSwap, permissionDeploySwap, (short) 3);
    swapFactory = blockchain.deployContract(creator, CONTRACT_BYTES, initRpc);

    // Update swap binary
    updateSwapBinary(
        creator,
        1_0_0,
        LiquiditySwapTest.CONTRACT_BYTES.code(),
        LiquiditySwapTest.CONTRACT_BYTES.abi());

    // Deploy swaps
    final DexSwapFactory.TokenPair tokenPair = tokenPair(token1, token2);
    final byte[] rpc = DexSwapFactory.deploySwapContract(tokenPair);
    blockchain.sendAction(creator, swapFactory, rpc);
    blockchain.sendAction(liquidityProvider, swapFactory, rpc);
    blockchain.sendAction(swapper, swapFactory, rpc);
  }

  private void transfer(
      BlockchainAddress contract, BlockchainAddress from, BlockchainAddress to, BigInteger amount) {
    final byte[] rpc = TokenContract.transfer(to, amount);
    blockchain.sendAction(from, contract, rpc);
  }

  private void approve(
      BlockchainAddress approver,
      BlockchainAddress contract,
      BlockchainAddress approvee,
      BigInteger amount) {
    final byte[] rpc = TokenContract.approve(approvee, amount);
    blockchain.sendAction(approver, contract, rpc);
  }

  private void deposit(BlockchainAddress depositor, BlockchainAddress token, BigInteger amount) {
    final byte[] rpc = LiquiditySwap.deposit(token, amount);
    blockchain.sendAction(depositor, swapAddress, rpc);
  }

  private void swap(
      BlockchainAddress swapper, BlockchainAddress inputToken, BigInteger inputAmount) {
    final byte[] rpc = LiquiditySwap.swap(inputToken, inputAmount, BigInteger.ONE);
    blockchain.sendAction(swapper, swapAddress, rpc);
  }

  private void provideInitialLiquidity(
      BlockchainAddress provider, BigInteger inputAmountTokenA, BigInteger inputAmountTokenB) {
    final byte[] rpc = LiquiditySwap.provideInitialLiquidity(inputAmountTokenA, inputAmountTokenB);
    blockchain.sendAction(provider, swapAddress, rpc);
  }

  private BigInteger swapDepositBalance(BlockchainAddress owner, BlockchainAddress tokenAddress) {
    final LiquiditySwap.LiquiditySwapContractState state =
        LiquiditySwap.LiquiditySwapContractState.deserialize(
            blockchain.getContractState(swapAddress));
    return LiquiditySwapTest.swapDepositBalance(state, owner, tokenAddress);
  }

  private BigInteger tokenBalance(BlockchainAddress owner, BlockchainAddress tokenAddress) {
    final TokenContract.TokenState state =
        TokenContract.TokenState.deserialize(blockchain.getContractState(tokenAddress));
    return state.balances().getOrDefault(owner, BigInteger.ZERO);
  }

  private void withdraw(BlockchainAddress owner, BlockchainAddress token, BigInteger amount) {
    final byte[] rpc = LiquiditySwap.withdraw(token, amount);
    blockchain.sendAction(owner, swapAddress, rpc);
  }

  private void reclaimLiquidity(BlockchainAddress owner, BigInteger amount) {
    final byte[] rpc = LiquiditySwap.reclaimLiquidity(amount);
    blockchain.sendAction(owner, swapAddress, rpc);
  }

  void updateSwapBinary(BlockchainAddress updater, int version, byte[] code, byte[] abi) {
    final byte[] rpc = DexSwapFactory.updateSwapBinary(code, abi, version);
    blockchain.sendAction(updater, swapFactory, rpc);
  }

  /**
   * Utility for creating sorted token pairs.
   *
   * <p>Sorted token pairs must be sorted like Addresses are sorted in Rust, where bytes are treated
   * as unsigned. This is different from BlockchainAddress, where bytes are sorted as signed.
   *
   * @param token1 First token
   * @param token2 Second token
   * @return Sorted token pair.
   */
  private static DexSwapFactory.TokenPair tokenPair(
      BlockchainAddress token1, BlockchainAddress token2) {
    // Emulate Rust sorting by converting to strings.
    final String[] addresses = new String[] {token1.writeAsString(), token2.writeAsString()};
    Arrays.sort(addresses);
    return new DexSwapFactory.TokenPair(
        BlockchainAddress.fromString(addresses[0]), BlockchainAddress.fromString(addresses[1]));
  }
}
