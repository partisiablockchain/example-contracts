package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkStatistics;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;

/** A test suite for the Zk statistics smart contract. */
public final class ZkStatisticsTest extends JunitContractTest {

  static final ContractBytes STATISTIC_CONTRACT =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_statistics.zkwa"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_statistics.abi"));

  BlockchainAddress statistics;
  BlockchainAddress owner;
  BlockchainAddress maleSurveyParticipant;
  BlockchainAddress femaleSurveyParticipant;
  BlockchainAddress otherSurveyParticipant;

  /** Deploy the contract with a standard deadline. */
  @ContractTest
  void setup() {
    owner = blockchain.newAccount(1);
    maleSurveyParticipant = blockchain.newAccount(2);
    femaleSurveyParticipant = blockchain.newAccount(3);
    otherSurveyParticipant = blockchain.newAccount(4);

    int inputTime = 20000;
    byte[] initialize = ZkStatistics.initialize(inputTime);

    statistics = blockchain.deployZkContract(owner, STATISTIC_CONTRACT, initialize);
    // The time is assigned and then increased by 1, so subtract 1 to get the deadline time.
    long blockProductionTime = blockchain.getBlockProductionTime() - 1;

    ZkStatistics.StatisticsContractState state =
        ZkStatistics.StatisticsContractState.deserialize(blockchain.getContractState(statistics));

    Assertions.assertThat(state.deadline()).isEqualTo(blockProductionTime + inputTime);
  }

  /** The owner can compute the statistics of multiple inputs. */
  @ContractTest(previous = "setup")
  void addThreeInputsAndCompute() {

    SurveyAnswer maleAnswer =
        new SurveyAnswer(AgeGroup.TWENTY_TO_THIRTY_NINE, Gender.Male, Color.GREEN);
    SurveyAnswer femaleAnswer =
        new SurveyAnswer(AgeGroup.TWENTY_TO_THIRTY_NINE, Gender.Female, Color.GREEN);
    SurveyAnswer otherAnswer = new SurveyAnswer(AgeGroup.SIXTY_PLUS, Gender.Other, Color.GREEN);

    blockchain.sendSecretInput(
        statistics, maleSurveyParticipant, createSecretFromAnswer(maleAnswer), secretInputRpc());
    blockchain.sendSecretInput(
        statistics,
        femaleSurveyParticipant,
        createSecretFromAnswer(femaleAnswer),
        secretInputRpc());
    blockchain.sendSecretInput(
        statistics, otherSurveyParticipant, createSecretFromAnswer(otherAnswer), secretInputRpc());

    blockchain.waitForBlockProductionTime(20005);

    byte[] computeStatistics = ZkStatistics.computeStatistics();
    blockchain.sendAction(owner, statistics, computeStatistics);

    ZkStatistics.StatisticsContractState state =
        ZkStatistics.StatisticsContractState.deserialize(blockchain.getContractState(statistics));

    ZkStatistics.StatisticsResult expectedOutput =
        new ZkStatistics.StatisticsResult(
            new ZkStatistics.AgeCounts(0, 2, 0, 1),
            new ZkStatistics.GenderCounts(1, 1, 1),
            new ZkStatistics.ColorCounts(0, 0, 3, 0));

    Assertions.assertThat(state.result()).isEqualTo(expectedOutput);
  }

  /** A user cannot add an input after the deadline. */
  @ContractTest(previous = "setup")
  void inputAfterDeadline() {
    blockchain.waitForBlockProductionTime(20005);

    SurveyAnswer maleAnswer =
        new SurveyAnswer(AgeGroup.TWENTY_TO_THIRTY_NINE, Gender.Male, Color.GREEN);

    Assertions.assertThatThrownBy(
            () ->
                blockchain.sendSecretInput(
                    statistics,
                    maleSurveyParticipant,
                    createSecretFromAnswer(maleAnswer),
                    secretInputRpc()))
        .hasMessageContaining(
            "Inputting data is not allowed after the deadline. Current time is 20006, deadline was"
                + " 20005");
  }

  /** The admin cannot compute the statistics before the deadline. */
  @ContractTest(previous = "setup")
  void startingTheComputationBeforeDeadline() {

    Assertions.assertThatThrownBy(
            () -> blockchain.sendAction(owner, statistics, ZkStatistics.computeStatistics()))
        .hasMessageContaining(
            "Cannot start computing statistics before deadline has been passed, deadline is 20005"
                + " ms UTC, current time is 7 ms UTC");
  }

  private CompactBitArray createSecretFromAnswer(SurveyAnswer answer) {
    return BitOutput.serializeBits(
        bitOutput -> {
          bitOutput.writeUnsignedInt(answer.ageGroup().surveyIdentifier, 8);
          bitOutput.writeUnsignedInt(answer.gender().surveyIdentifier, 8);
          bitOutput.writeUnsignedInt(answer.color().surveyIdentifier, 8);
        });
  }

  /** Matching representation to the contract definition of age groups. */
  private enum AgeGroup {
    ZERO_TO_NINETEEN(1),
    TWENTY_TO_THIRTY_NINE(2),
    FORTY_TO_FIFTY_NINE(3),
    SIXTY_PLUS(4);

    private final int surveyIdentifier;

    AgeGroup(int surveyIdentifier) {
      this.surveyIdentifier = surveyIdentifier;
    }
  }

  /** Matching representation to the contract definition of genders. */
  private enum Gender {
    Male(1),
    Female(2),
    Other(3);

    private final int surveyIdentifier;

    Gender(int surveyGenderIdentifier) {
      this.surveyIdentifier = surveyGenderIdentifier;
    }
  }

  /** Matching representation to the contract definition of colors that can be answered. */
  private enum Color {
    RED(1),
    BLUE(2),
    GREEN(3),
    YELLOW(4);

    private final int surveyIdentifier;

    Color(int surveyColorIdentifier) {
      this.surveyIdentifier = surveyColorIdentifier;
    }
  }

  private record SurveyAnswer(AgeGroup ageGroup, Gender gender, Color color) {}

  byte[] secretInputRpc() {
    return new byte[] {0x40};
  }
}
