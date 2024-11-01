package examples;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkClassification;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.FuzzyState;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.testenvironment.zk.node.ZkComputationComplexity;
import com.secata.stream.BitInput;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import com.secata.stream.SafeDataOutputStream;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;
import org.assertj.core.api.Assertions;

/** Test for ZK Decision Tree Evaluation Contract. */
public final class ZkClassificationTest extends JunitContractTest {

  private BlockchainAddress modelOwner;
  private BlockchainAddress sampleOwner;
  private BlockchainAddress resultReceiver;
  private BlockchainAddress classifier;

  private static final ContractBytes CONTRACT_BYTES =
      ContractBytes.fromPbcFile(
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_classification.pbc"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/zk_classification_runner"));

  // scaling between fixed points to ensure precision in value/threshold comparison
  private final int[] conversion = new int[] {1, 1, 1, 1, 1, 1, 1, 1, 1, 1};

  // example of an input sample (inference result is 1 given example model)
  private final double[] input = new double[] {42, 10, 6000, 0, 37, 0, 1, 0, 1, 1};

  /** Deploys contract. */
  @ContractTest
  void deploy() {
    modelOwner = blockchain.newAccount(1);
    sampleOwner = blockchain.newAccount(2);
    resultReceiver = blockchain.newAccount(3);

    byte[] initRpc = ZkClassification.initialize();
    classifier = blockchain.deployZkContract(modelOwner, CONTRACT_BYTES, initRpc);
  }

  /** Only contract sender (model owner) can add model. */
  @ContractTest(previous = "deploy")
  public void addSecretModelNonOwner() {
    CompactBitArray model = secretModel();
    byte[] modelRpc = addModelPublicRpc(conversion);

    Assertions.assertThatThrownBy(
            () -> blockchain.sendSecretInput(classifier, sampleOwner, model, modelRpc))
        .isInstanceOf(RuntimeException.class)
        .hasMessageContaining("Only contract creator can add a model");
  }

  /** Input sample can only be added after model has been added. */
  @ContractTest(previous = "deploy")
  public void addSecretSampleBeforeModel() {
    CompactBitArray sample = secretSample(input);
    byte[] sampleRpc = addSamplePublicRpc(1, sampleOwner);

    Assertions.assertThatThrownBy(
            () -> blockchain.sendSecretInput(classifier, sampleOwner, sample, sampleRpc))
        .isInstanceOf(RuntimeException.class);
  }

  /**
   * ZK computation complexity analysis showing the number of multiplications and the number of
   * rounds needed to evaluate the model on a single input sample.
   */
  @ContractTest(previous = "deploy")
  public void computationComplexity() {
    CompactBitArray model = secretModel();
    byte[] modelRpc = addModelPublicRpc(conversion);
    blockchain.sendSecretInput(classifier, modelOwner, model, modelRpc);

    CompactBitArray sample = secretSample(input);
    byte[] sampleRpc = addSamplePublicRpc(1, resultReceiver);
    blockchain.sendSecretInput(classifier, sampleOwner, sample, sampleRpc);

    FuzzyState zkState = blockchain.getContractStateJson(classifier);
    int variableId = zkState.getNode("/variables").size();
    byte[] result = zkNodes.getSecretVariable(classifier, variableId).data();
    Assertions.assertThat(BitInput.create(result).readBoolean()).isEqualTo(true);

    ZkComputationComplexity complexity = zkNodes.getComplexityOfLastComputation();
    int multiplications = complexity.multiplicationCount();
    int rounds = complexity.numberOfRounds();

    Assertions.assertThat(multiplications).isEqualTo(2968);
    Assertions.assertThat(rounds).isEqualTo(201);
  }

  /**
   * The input sample owner specifies who the recipient of the evaluation result is. When the
   * computation is complete, the ownership of the result is transferred to said recipient. Only
   * they recipient can access it.
   */
  @ContractTest(previous = "deploy")
  public void transferOwnershipOfResult() {
    CompactBitArray model = secretModel();
    byte[] modelRpc = addModelPublicRpc(conversion);
    blockchain.sendSecretInput(classifier, modelOwner, model, modelRpc);

    CompactBitArray sample = secretSample(input);
    byte[] sampleRpc = addSamplePublicRpc(1, resultReceiver);
    blockchain.sendSecretInput(classifier, sampleOwner, sample, sampleRpc);

    FuzzyState zkState = blockchain.getContractStateJson(classifier);
    int resultId = zkState.getNode("/variables").size();
    String resultOwner = zkState.getNode("/variables/" + (resultId - 1) + "/value/owner").asText();

    Assertions.assertThat(resultOwner).isNotEqualTo(sampleOwner.writeAsString());
    Assertions.assertThat(resultOwner).isEqualTo(resultReceiver.writeAsString());
  }

  /**
   * The secret-shared model computes the same output as the same model would in a normal evaluation
   * computed with the same input.
   *
   * <p>Compares the outputs of the secret-shared model (taking secret-shared samples as evaluation
   * input) to the outputs of the non-secret version of the model (taking the same but non-secret
   * samples as evaluation input). Predictions should be identical for all 200 samples.
   */
  @ContractTest(previous = "deploy")
  public void compareAccuracyToInputtedModel() {
    List<CompactBitArray> samples = getTestSamples();
    List<Boolean> predictions = getModelPredictionsFromTraining();

    // model owner inputs secret model
    CompactBitArray model = secretModel();
    byte[] modelRpc = addModelPublicRpc(conversion);
    blockchain.sendSecretInput(classifier, modelOwner, model, modelRpc);

    // sample owner inputs secret samples (takes about 30 seconds to finish)
    for (int i = 0; i < samples.size(); i++) {
      CompactBitArray sample = samples.get(i);
      byte[] sampleRpc = addSamplePublicRpc(1, resultReceiver);
      blockchain.sendSecretInput(classifier, sampleOwner, sample, sampleRpc);

      FuzzyState zkState = blockchain.getContractStateJson(classifier);
      int variableId = zkState.getNode("/variables").size();
      byte[] result = zkNodes.getSecretVariable(classifier, variableId).data();

      Assertions.assertThat(BitInput.create(result).readBoolean()).isEqualTo(predictions.get(i));
    }
  }

  private static byte[] addModelPublicRpc(int[] conversion) {
    return SafeDataOutputStream.serialize(
        safeDataOutputStream -> {
          safeDataOutputStream.writeByte(0x040);
          safeDataOutputStream.writeInt(conversion.length);
          for (int i : conversion) {
            safeDataOutputStream.writeShort(i);
          }
        });
  }

  private static byte[] addSamplePublicRpc(int modelId, BlockchainAddress resultReceiver) {
    return SafeDataOutputStream.serialize(
        safeDataOutputStream -> {
          safeDataOutputStream.writeByte(0x041);
          safeDataOutputStream.writeInt(modelId);
          resultReceiver.write(safeDataOutputStream);
        });
  }

  private CompactBitArray secretModel() {
    String path = "/zk-classification-model.json";

    try (InputStream stream = ZkClassificationTest.class.getResourceAsStream(path)) {
      ObjectMapper mapper = new ObjectMapper();
      DeserializedModel deserializedModel = mapper.readValue(stream, DeserializedModel.class);
      ZkClassification.Model secretModel = getZkModel(deserializedModel);

      return BitOutput.serializeBits(
          output -> {
            writeInternalVertices(output, secretModel);
            writeLeafVertices(output, secretModel);
          });
    } catch (IOException e) {
      throw new RuntimeException(e);
    }
  }

  private ZkClassification.Model getZkModel(DeserializedModel model) {
    List<ZkClassification.InternalVertex> internals = new ArrayList<>();
    for (int i = 0; i < model.internals().size(); i++) {
      int feature = model.internals.get(i).feature;
      double threshold = model.internals.get(i).threshold * conversion[feature];
      internals.add(new ZkClassification.InternalVertex((byte) feature, (short) threshold));
    }

    List<ZkClassification.LeafVertex> leaves = new ArrayList<>();
    for (int i = 0; i < model.leaves.size(); i++) {
      leaves.add(new ZkClassification.LeafVertex(model.leaves.get(i).classification));
    }

    return new ZkClassification.Model(internals, leaves);
  }

  private CompactBitArray secretSample(double[] values) {
    List<Short> converted = new ArrayList<>();
    for (int i = 0; i < values.length; i++) {
      converted.add((short) (values[i] * conversion[i]));
    }

    ZkClassification.Sample sample = new ZkClassification.Sample(converted);

    return BitOutput.serializeBits(
        output -> {
          for (Short value : sample.values()) {
            output.writeSignedInt(value, 16);
          }
        });
  }

  private List<CompactBitArray> getTestSamples() {
    List<CompactBitArray> samples = new ArrayList<>();
    String path = "/zk-classification-test-samples.txt";

    try (InputStream stream = ZkClassificationTest.class.getResourceAsStream(path)) {
      BufferedReader reader =
          new BufferedReader(
              new InputStreamReader(Objects.requireNonNull(stream), StandardCharsets.UTF_8));

      String line;
      while ((line = reader.readLine()) != null) {
        double[] values = Arrays.stream(line.split(",")).mapToDouble(Double::parseDouble).toArray();
        CompactBitArray sample = secretSample(Arrays.stream(values, 0, values.length).toArray());
        samples.add(sample);
      }
    } catch (IOException e) {
      throw new RuntimeException(e);
    }

    return samples;
  }

  private List<Boolean> getModelPredictionsFromTraining() {
    List<Boolean> predictions = new ArrayList<>();
    String path = "/zk-classification-predictions.txt";

    try (InputStream stream = ZkClassificationTest.class.getResourceAsStream(path)) {
      BufferedReader reader =
          new BufferedReader(
              new InputStreamReader(Objects.requireNonNull(stream), StandardCharsets.UTF_8));

      String line;
      while ((line = reader.readLine()) != null) {
        int prediction = Integer.parseInt(line);
        predictions.add(prediction != 0);
      }
    } catch (IOException e) {
      throw new RuntimeException(e);
    }

    return predictions;
  }

  private static void writeInternalVertices(BitOutput output, ZkClassification.Model model) {
    for (ZkClassification.InternalVertex vertex : model.internals()) {
      output.writeUnsignedInt(vertex.feature(), 8);
      output.writeSignedInt(vertex.threshold(), 16);
    }
  }

  private static void writeLeafVertices(BitOutput output, ZkClassification.Model model) {
    for (ZkClassification.LeafVertex leaf : model.leaves()) {
      output.writeBoolean(leaf.classification());
    }
  }

  private record DeserializedModel(
      List<DeserializedInternalVertex> internals, List<DeserializedLeafVertex> leaves) {}

  private record DeserializedInternalVertex(int feature, double threshold) {}

  private record DeserializedLeafVertex(boolean classification) {}
}
