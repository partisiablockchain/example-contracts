package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.MiaGame;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.Previous;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.language.junit.exceptions.SecretInputFailureException;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import org.assertj.core.api.Assertions;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

/** A test suite for the Mia Game smart contract. */
public final class MiaGameTest extends JunitContractTest {

  /** {@link MiaGame} contract bytes. */
  private static final ContractBytes MIA_CONTRACT =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/mia_game.zkwa"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/mia_game.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/mia_game_runner"));

  private static final DiceThrowPoints THIRTY_TWO =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 1, (byte) 2), 3);
  private static final DiceThrowPoints FORTY_ONE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 3, (byte) 0), 4);
  private static final DiceThrowPoints FORTY_TWO =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 3, (byte) 1), 5);
  private static final DiceThrowPoints FORTY_THREE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 3, (byte) 2), 6);
  private static final DiceThrowPoints FIFTY_ONE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 4, (byte) 0), 8);
  private static final DiceThrowPoints FIFTY_TWO =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 4, (byte) 1), 9);
  private static final DiceThrowPoints FIFTY_THREE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 4, (byte) 2), 10);
  private static final DiceThrowPoints FIFTY_FOUR =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 4, (byte) 3), 12);
  private static final DiceThrowPoints SIXTY_ONE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 5, (byte) 0), 16);
  private static final DiceThrowPoints SIXTY_TWO =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 5, (byte) 1), 17);
  private static final DiceThrowPoints SIXTY_THREE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 5, (byte) 2), 18);
  private static final DiceThrowPoints SIXTY_FOUR =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 5, (byte) 3), 20);
  private static final DiceThrowPoints SIXTY_FIVE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 5, (byte) 4), 24);
  private static final DiceThrowPoints PAIR_ONE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 0, (byte) 0), 32);
  private static final DiceThrowPoints PAIR_TWO =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 1, (byte) 1), 33);
  private static final DiceThrowPoints PAIR_THREE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 2, (byte) 2), 34);
  private static final DiceThrowPoints PAIR_FOUR =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 3, (byte) 3), 38);
  private static final DiceThrowPoints PAIR_FIVE =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 4, (byte) 4), 40);
  private static final DiceThrowPoints PAIR_SIX =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 5, (byte) 5), 48);
  private static final DiceThrowPoints LITTLE_MIA =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 0, (byte) 2), 66);
  private static final DiceThrowPoints MIA =
      new DiceThrowPoints(new MiaGame.DiceThrow((byte) 0, (byte) 1), 129);

  private static final List<DiceThrowPoints> possibleThrows =
      new ArrayList<>(
          Arrays.asList(
              THIRTY_TWO,
              FORTY_ONE,
              FORTY_TWO,
              FORTY_THREE,
              FIFTY_ONE,
              FIFTY_TWO,
              FIFTY_THREE,
              FIFTY_FOUR,
              SIXTY_ONE,
              SIXTY_TWO,
              SIXTY_THREE,
              SIXTY_FOUR,
              SIXTY_FIVE,
              PAIR_ONE,
              PAIR_TWO,
              PAIR_THREE,
              PAIR_FOUR,
              PAIR_FIVE,
              PAIR_SIX,
              MIA,
              LITTLE_MIA));
  private BlockchainAddress game;
  private BlockchainAddress player1;
  private BlockchainAddress player2;
  private BlockchainAddress player3;

  /**
   * When a game is deployed with three players, the game phase is 'Start', and the player who
   * deploys the contract is in turn, and all players have 3 lives.
   */
  @ContractTest
  void deploy() {
    player1 = blockchain.newAccount(1);
    player2 = blockchain.newAccount(2);
    player3 = blockchain.newAccount(3);

    byte[] initRpc = MiaGame.initialize(List.of(player1, player2, player3));

    game = blockchain.deployZkContract(player1, MIA_CONTRACT, initRpc);

    assertNumberOfPlayersLeft(3);
    assertPlayerInTurn(player1);
    assertCurrentGamePhase(MiaGame.GamePhaseD.START);
    assertPlayersNumberOfLivesLeft(player1, 6);
  }

  /** When the player in turn starts a round of the game, the game phase is 'Add Randomness'. */
  @ContractTest(previous = "deploy")
  void startTheGame() {
    byte[] startRpc = MiaGame.startRound();
    blockchain.sendAction(player1, game, startRpc);

    assertNumberOfContributions(0);
    assertCurrentGamePhase(MiaGame.GamePhaseD.ADD_RANDOMNESS);
    assertPlayersNumberOfLivesLeft(player1, 6);
  }

  /** Many people can participate in a game. */
  @ContractTest
  void deployWithManyPlayers() {
    player1 = blockchain.newAccount(1);
    player2 = blockchain.newAccount(2);
    player3 = blockchain.newAccount(3);
    BlockchainAddress player4 = blockchain.newAccount(4);
    BlockchainAddress player5 = blockchain.newAccount(5);
    BlockchainAddress player6 = blockchain.newAccount(6);
    BlockchainAddress player7 = blockchain.newAccount(7);

    byte[] initRpc =
        MiaGame.initialize(List.of(player1, player2, player3, player4, player5, player6, player7));

    game = blockchain.deployZkContract(player1, MIA_CONTRACT, initRpc);

    assertNumberOfPlayersLeft(7);
    assertPlayerInTurn(player1);
    assertCurrentGamePhase(MiaGame.GamePhaseD.START);
  }

  /**
   * All players can add randomness, and the game phase is 'Throw' when all players have contributed
   * to the randomness.
   */
  @ContractTest(previous = "startTheGame")
  void addRandomnessForFirstThrow() {
    addRandomness(player1, 0, 3);
    addRandomness(player2, 1, 4);

    assertNumberOfContributions(2);
    assertCurrentGamePhase(MiaGame.GamePhaseD.ADD_RANDOMNESS);

    // Add the last contribution.
    addRandomness(player3, 2, 5);

    assertNumberOfContributions(0);
    assertCurrentGamePhase(MiaGame.GamePhaseD.THROW);
  }

  /** A player can add a randomness contribution which is greater than or equal to 6. */
  @ContractTest(previous = "startTheGame")
  void addRandomnessGreaterThan6() {
    addRandomness(player1, 10, 0);
    addRandomness(player2, 6, 12);
    addRandomness(player3, 80, 56);

    assertCurrentGamePhase(MiaGame.GamePhaseD.THROW);
  }

  /** The player in turn can throw the dice when all players have added randomness. */
  @ContractTest(previous = "addRandomnessForFirstThrow")
  void throwDice() {
    callThrowDice(player1);
    assertCurrentGamePhase(MiaGame.GamePhaseD.ANNOUNCE);
    assertNumberOfContributions(0);
  }

  /**
   * The throwing player can announce their throw, which changes the phase, so the next player must
   * decide whether they believe the throwing player.
   */
  @ContractTest(previous = "throwDice")
  void announce() {
    announceDiceValues(player1, 3, 2);
    assertCurrentGamePhase(MiaGame.GamePhaseD.DECIDE);
    assertNumberOfContributions(0);
  }

  /**
   * Believing the previous player changes the game phase i.e. starts a new round and resets
   * randomness contributions.
   */
  @ContractTest(previous = "announce")
  void believe() {
    callBelieve(player2);
    assertCurrentGamePhase(MiaGame.GamePhaseD.ADD_RANDOMNESS);
    assertNumberOfContributions(0);
    assertPlayerInTurn(player2);
  }

  /**
   * When a player lies about their throw (not Mia) and is called out by the next player, the lying
   * player loses one life.
   */
  @ContractTest(previous = "believe")
  void playerLying() {
    final int player2Lives = getPlayerLives(player2);

    specificThrow(1, 4);
    callThrowDice(player2);

    assertCurrentGamePhase(MiaGame.GamePhaseD.ANNOUNCE);
    assertPlayerInTurn(player2);
    announceDiceValues(player2, 5, 5);

    assertCurrentGamePhase(MiaGame.GamePhaseD.DECIDE);
    calloutPlayer(player3);

    assertRevealedThrow(1, 4);
    assertPlayersNumberOfLivesLeft(player2, player2Lives - 1);
  }

  /**
   * A round where the throw stated is Mia and the throw is actually Mia, makes the player calling
   * the thrower a liar lose 2 lives.
   */
  @ContractTest(previous = "startTheGame")
  void playerGetsMia() {
    final int player2Lives = getPlayerLives(player2);

    throwMia();
    callThrowDice(player1);

    announceDiceValues(player1, 0, 1);
    calloutPlayer(player2);

    assertRevealedThrow(1, 0);
    assertPlayersNumberOfLivesLeft(player2, player2Lives - 2);
  }

  /**
   * A round where the throw stated is Mia and the throw is not Mia, makes lying player lose 2
   * lives.
   */
  @ContractTest(previous = "startTheGame")
  void playerLiesAboutMia() {
    final int player1Lives = getPlayerLives(player1);

    specificThrow(2, 4);
    callThrowDice(player1);
    announceDiceValues(player1, 0, 1);
    calloutPlayer(player2);

    assertRevealedThrow(2, 4);
    assertPlayersNumberOfLivesLeft(player1, player1Lives - 2);
  }

  /** When a player loses all of their lives, they are out of the game. */
  @ContractTest(previous = "startTheGame")
  void playerLoses() {
    throwMia();
    callThrowDice(player1);
    announceDiceValues(player1, 0, 1);
    calloutPlayer(player2);

    assertPlayerInTurn(player2);
    assertPlayersNumberOfLivesLeft(player2, 4);

    specificThrow(0, 0);
    callThrowDice(player2);
    announceDiceValues(player2, 0, 1);
    calloutPlayer(player3);

    assertPlayerInTurn(player3);
    assertPlayersNumberOfLivesLeft(player2, 2);

    throwMia();
    callThrowDice(player3);
    announceDiceValues(player3, 0, 1);
    callBelieve(player1);

    throwMia();
    callThrowDice(player1);
    announceDiceValues(player1, 0, 1);
    calloutPlayer(player2);

    assertPlayerInTurn(player3);

    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));

    Assertions.assertThat(state.players().size()).isEqualTo(2);
    Assertions.assertThat(state.players()).doesNotContain(player2);
  }

  /** A player wins the game when only he or she has more lives left. */
  @ContractTest(previous = "playerLoses")
  void playerWins() {
    assertNumberOfPlayersLeft(2);

    throwMia();
    callThrowDice(player3);
    announceDiceValues(player3, 0, 1);
    calloutPlayer(player1);

    assertPlayersNumberOfLivesLeft(player1, 4);

    specificThrow(0, 3);
    callThrowDice(player1);
    announceDiceValues(player1, 0, 1);
    calloutPlayer(player3);

    assertPlayersNumberOfLivesLeft(player1, 2);

    throwMia();
    callThrowDice(player3);
    announceDiceValues(player3, 0, 1);
    calloutPlayer(player1);

    assertPlayersNumberOfLivesLeft(player1, 0);
    assertNumberOfPlayersLeft(1);

    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));

    Assertions.assertThat(state.gamePhase().discriminant()).isEqualTo(MiaGame.GamePhaseD.DONE);
    Assertions.assertThat(state.winner()).isEqualTo(player3);
  }

  /** The contract cannot be deployed with less than 3 players. */
  @ContractTest
  void deployNotEnoughPlayers() {
    player1 = blockchain.newAccount(1);
    player2 = blockchain.newAccount(2);

    byte[] initRpc = MiaGame.initialize(List.of(player1, player2));

    Assertions.assertThatThrownBy(() -> blockchain.deployZkContract(player1, MIA_CONTRACT, initRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("There must be at least 3 players to play Mia.");
  }

  /** The same player cannot join the same game more than once. */
  @ContractTest
  void deploySamePlayerTwice() {
    player1 = blockchain.newAccount(1);
    player2 = blockchain.newAccount(2);

    byte[] initRpc = MiaGame.initialize(List.of(player1, player2, player2));

    Assertions.assertThatThrownBy(() -> blockchain.deployZkContract(player1, MIA_CONTRACT, initRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("No duplicates in players.");
  }

  /** A player who is not in turn cannot start a new round. */
  @ContractTest(previous = "deploy")
  void wrongPlayerStartsRound() {
    byte[] startRpc = MiaGame.startRound();

    Assertions.assertThatThrownBy(() -> blockchain.sendAction(player2, game, startRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only the player whose turn it is can start the round.");
  }

  /** A player cannot add randomness when the game phase is not 'Add Randomness'. */
  @ContractTest(previous = "deploy")
  void addRandomnessInWrongState() {
    assertCurrentGamePhase(MiaGame.GamePhaseD.START);

    Assertions.assertThatThrownBy(() -> addRandomness(player1, 1, 1))
        .isInstanceOf(SecretInputFailureException.class)
        .hasMessageContaining("Must be in the AddRandomness phase to input secret randomness.");
  }

  /** The same player cannot add randomness more than once. */
  @ContractTest(previous = "startTheGame")
  void addRandomnessTwice() {
    assertCurrentGamePhase(MiaGame.GamePhaseD.ADD_RANDOMNESS);

    addRandomness(player1, 0, 0);

    Assertions.assertThatThrownBy(() -> addRandomness(player1, 1, 1))
        .isInstanceOf(SecretInputFailureException.class)
        .hasMessageContaining(
            "Each Player is only allowed to send one contribution to the randomness of the dice"
                + " throw.");
  }

  /** A player cannot throw dice when the game phase is not in the Throw phase. */
  @ContractTest(previous = "startTheGame")
  void throwDiceInRandomnessPhase() {
    assertCurrentGamePhase(MiaGame.GamePhaseD.ADD_RANDOMNESS);
    assertPlayerInTurn(player1);

    Assertions.assertThatThrownBy(() -> callThrowDice(player1))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("The dice can only be thrown in the Throw phase");
  }

  /** A player not in turn cannot throw dice. */
  @ContractTest(previous = "addRandomnessForFirstThrow")
  void playerNotInTurnThrowsDice() {
    assertCurrentGamePhase(MiaGame.GamePhaseD.THROW);
    assertPlayerInTurn(player1);

    Assertions.assertThatThrownBy(() -> callThrowDice(player2))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only the player in turn can throw the dice.");
  }

  /** A player not in turn cannot state the value of the dice throw. */
  @ContractTest(previous = "throwDice")
  void playerNotInTurnStatesDiceThrow() {
    assertPlayerInTurn(player1);

    Assertions.assertThatThrownBy(() -> announceDiceValues(player2, 3, 2))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only the current player can state the value of the dice throw.");
  }

  /** The stated throw cannot be lower than the previously stated throw. */
  @ContractTest(previous = "startTheGame")
  void statedThrowTooLow() {
    specificThrow(5, 5);
    callThrowDice(player1);
    announceDiceValues(player1, 5, 5);
    callBelieve(player2);

    assertThrowToBeat(PAIR_SIX.diceThrow());

    specificThrow(3, 4);
    callThrowDice(player2);

    Assertions.assertThatThrownBy(() -> announceDiceValues(player2, 3, 4))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Stated throw must be better than the last stated throw.");
  }

  /** A player who is not next in turn cannot believe the current player's throw. */
  @ContractTest(previous = "startTheGame")
  void wrongPlayerBelievesThrow() {
    specificThrow(5, 5);
    callThrowDice(player1);
    announceDiceValues(player1, 5, 5);

    Assertions.assertThatThrownBy(() -> callBelieve(player3))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only the next player can say if they believe the stated throw.");
  }

  /**
   * The next player cannot say that they believe the dice throw if the game phase is not 'Decide'.
   */
  @ContractTest(previous = "startTheGame")
  void believeInWrongPhase() {
    assertPlayerInTurn(player1);
    assertCurrentGamePhase(MiaGame.GamePhaseD.ADD_RANDOMNESS);

    Assertions.assertThatThrownBy(() -> callBelieve(player2))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Must be in the deciding phase to say believe.");
  }

  /** A player who is not next in turn cannot call out the current player's throw. */
  @ContractTest(previous = "startTheGame")
  void wrongPlayerCallsOutThrow() {
    specificThrow(5, 5);
    callThrowDice(player1);
    announceDiceValues(player1, 5, 5);

    Assertions.assertThatThrownBy(() -> calloutPlayer(player3))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Only the next player can say if the throwing player is lying.");
  }

  /** The next player cannot call out the dice throw if the game phase is not 'Decide'. */
  @ContractTest(previous = "startTheGame")
  void callOutWhenInWrongPhase() {
    assertPlayerInTurn(player1);
    assertCurrentGamePhase(MiaGame.GamePhaseD.ADD_RANDOMNESS);
    Assertions.assertThatThrownBy(() -> calloutPlayer(player2))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Must be in the deciding phase say if the throwing player is lying.");
  }

  /** A higher dice throw wins against a lower or equal dice throw. */
  @Previous(value = "startTheGame")
  @ParameterizedTest
  @MethodSource("providePairThrow")
  void throwCombinations(PairThrow pairThrow) {
    DiceThrowPoints throwToBeat = pairThrow.toBeat();
    specificMiaDiceThrow(throwToBeat.diceThrow());
    callThrowDice(player1);
    announceMiaDiceValues(player1, throwToBeat.diceThrow());
    callBelieve(player2);

    DiceThrowPoints nextThrow = pairThrow.attempt();
    specificMiaDiceThrow(nextThrow.diceThrow());
    callThrowDice(player2);
    byte[] announceThrow = MiaGame.announceThrow(nextThrow.diceThrow());

    // An exception is thrown if the next throw isn't better or equal to the throw to beat
    if (throwToBeat.throwValue() > nextThrow.throwValue()) {
      Assertions.assertThatThrownBy(() -> blockchain.sendAction(player2, game, announceThrow))
          .isInstanceOf(ActionFailureException.class)
          .hasMessageContaining("Stated throw must be better than the last stated throw.");
    } else {
      Assertions.assertThatNoException()
          .isThrownBy(() -> blockchain.sendAction(player2, game, announceThrow));
    }
  }

  private static List<PairThrow> providePairThrow() {
    // A list of all possible combinations of two dice throws.
    List<PairThrow> result = new ArrayList<>();
    for (DiceThrowPoints toBeat : possibleThrows) {
      for (DiceThrowPoints attempt : possibleThrows) {
        result.add(new PairThrow(toBeat, attempt));
      }
    }
    return result;
  }

  private void assertRevealedThrow(int d1, int d2) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    Assertions.assertThat(state.throwResult())
        .isEqualTo(new MiaGame.DiceThrow((byte) d1, (byte) d2));
  }

  private void calloutPlayer(BlockchainAddress sender) {
    blockchain.sendAction(sender, game, MiaGame.callOut());
  }

  private void announceDiceValues(BlockchainAddress sender, int d1, int d2) {
    byte[] announceThrow = MiaGame.announceThrow(new MiaGame.DiceThrow((byte) d1, (byte) d2));
    blockchain.sendAction(sender, game, announceThrow);
  }

  private void announceMiaDiceValues(BlockchainAddress player, MiaGame.DiceThrow diceThrow) {
    int d1 = diceThrow.d1();
    int d2 = diceThrow.d2();
    announceDiceValues(player, d1, d2);
  }

  private void callThrowDice(BlockchainAddress sender) {
    byte[] throwRpc = MiaGame.throwDice();
    blockchain.sendAction(sender, game, throwRpc);
  }

  private void callBelieve(BlockchainAddress sender) {
    byte[] believeRpc = MiaGame.believe();
    blockchain.sendAction(sender, game, believeRpc);
  }

  CompactBitArray createSecretInput(int d1, int d2) {
    return BitOutput.serializeBits(
        output -> {
          output.writeUnsignedInt(d1, 8);
          output.writeUnsignedInt(d2, 8);
        });
  }

  void assertCurrentGamePhase(MiaGame.GamePhaseD phase) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    Assertions.assertThat(state.gamePhase().discriminant()).isEqualTo(phase);
  }

  void assertPlayerInTurn(BlockchainAddress playerAddress) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    Assertions.assertThat(state.players().get(state.playerThrowing())).isEqualTo(playerAddress);
  }

  void assertNumberOfContributions(int expectedNumber) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    Assertions.assertThat(state.nrOfRandomnessContributions()).isEqualTo(expectedNumber);
  }

  void assertPlayersNumberOfLivesLeft(BlockchainAddress player, int numberOfLives) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    Assertions.assertThat(state.playerLives().get(player)).isEqualTo((byte) numberOfLives);
  }

  private void assertThrowToBeat(MiaGame.DiceThrow diceThrow) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    Assertions.assertThat(state.throwToBeat()).isEqualTo(diceThrow);
  }

  private void assertNumberOfPlayersLeft(int expectedNumber) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    Assertions.assertThat(state.players().size()).isEqualTo(expectedNumber);
  }

  void addRandomness(BlockchainAddress sender, int d1, int d2) {
    blockchain.sendSecretInput(game, sender, createSecretInput(d1, d2), new byte[] {0x40});
  }

  private int getPlayerLives(BlockchainAddress player) {
    MiaGame.MiaState state = MiaGame.MiaState.deserialize(blockchain.getContractState(game));
    return state.playerLives().get(player);
  }

  void throwMia() {
    addRandomness(player1, 0, 0);
    addRandomness(player2, 1, 0);
    addRandomness(player3, 0, 0);
  }

  void specificThrow(int d1, int d2) {
    addRandomness(player1, d1, d2);
    addRandomness(player2, 0, 0);
    addRandomness(player3, 0, 0);
  }

  private void specificMiaDiceThrow(MiaGame.DiceThrow diceThrow) {
    int d1 = diceThrow.d1();
    int d2 = diceThrow.d2();
    specificThrow(d1, d2);
  }

  /**
   * A pair af throws.
   *
   * @param toBeat the throw that the attempt must be better than.
   * @param attempt the attempted throw.
   */
  record PairThrow(DiceThrowPoints toBeat, DiceThrowPoints attempt) {}

  /**
   * A throw of two dices and its associated value.
   *
   * @param diceThrow The dice throw.
   * @param throwValue The value of the dice throw in the Mia contract's point system.
   */
  record DiceThrowPoints(MiaGame.DiceThrow diceThrow, int throwValue) {}
}
