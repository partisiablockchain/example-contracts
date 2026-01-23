package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.container.execution.protocol.HttpResponseData;
import com.partisiablockchain.crypto.KeyPair;
import com.partisiablockchain.language.abicodegen.OffChainFetchExchangeRate;
import com.partisiablockchain.language.abicodegen.OffChainFetchExchangeRate.ExchangeRates;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import com.partisiablockchain.language.testenvironment.executionengine.TestExecutionEngine;
import com.secata.tools.immutable.Bytes;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import org.assertj.core.api.Assertions;

/** {@link FetchExchangeRate} testing. */
public final class OffChainFetchExchangeRateTest extends JunitContractTest {

  /** Private keys for the execution engine. */
  public static final KeyPair ENGINE_KEY = new KeyPair(BigInteger.valueOf(20L));

  private TestExecutionEngine executionEngine;
  private BlockchainAddress engineAddress;

  private static final ContractBytes CONTRACT_BYTES =
      ContractBytesLoader.forContract("off_chain_fetch_exchange_rate");

  private BlockchainAddress contractAddress;
  private BlockchainAddress sender;

  private static final String EXAMPLE_EXCHANGE_RATES =
      """
        <gesmes:Envelope>
          <gesmes:subject>Reference rates</gesmes:subject>
          <gesmes:Sender>
            <gesmes:name>European Central Bank</gesmes:name>
          </gesmes:Sender>
          <Cube>
            <Cube time='2025-10-21'>
            <Cube currency='USD' rate='1.1607'/>
            <Cube currency='JPY' rate='176.45'/>
            <Cube currency='BGN' rate='1.9558'/>
            <Cube currency='CZK' rate='24.314'/>
            <Cube currency='DKK' rate='7.4691'/>
            <Cube currency='GBP' rate='0.86780'/>
            <Cube currency='HUF' rate='389.63'/>
            <Cube currency='PLN' rate='4.2393'/>
            <Cube currency='RON' rate='5.0838'/>
            <Cube currency='SEK' rate='10.9345'/>
            <Cube currency='CHF' rate='0.9230'/>
            <Cube currency='ISK' rate='141.60'/>
            <Cube currency='NOK' rate='11.6693'/>
            <Cube currency='TRY' rate='48.7114'/>
            <Cube currency='AUD' rate='1.7885'/>
            <Cube currency='BRL' rate='6.2526'/>
            <Cube currency='CAD' rate='1.6301'/>
            <Cube currency='CNY' rate='8.2626'/>
            <Cube currency='HKD' rate='9.0204'/>
            <Cube currency='IDR' rate='19272.67'/>
            <Cube currency='ILS' rate='3.8174'/>
            <Cube currency='INR' rate='102.1073'/>
            <Cube currency='KRW' rate='1660.38'/>
            <Cube currency='MXN' rate='21.4006'/>
            <Cube currency='MYR' rate='4.9086'/>
            <Cube currency='NZD' rate='2.0287'/>
            <Cube currency='PHP' rate='67.761'/>
            <Cube currency='SGD' rate='1.5061'/>
            <Cube currency='THB' rate='38.071'/>
            <Cube currency='ZAR' rate='20.1148'/>
            </Cube>
          </Cube>
      """;

  private static final String EXAMPLE_EXCHANGE_RATES_WITHOUT_ENTRIES =
      """
        <gesmes:Envelope>
          <gesmes:subject>Reference rates</gesmes:subject>
          <gesmes:Sender>
            <gesmes:name>European Central Bank</gesmes:name>
          </gesmes:Sender>
          <Cube>
            <Cube time='2025-10-21'>
            </Cube>
          </Cube>
      """;

  /** The contract can be deployed. */
  @ContractTest
  void deploy() {
    sender = blockchain.newAccount(3);

    engineAddress = blockchain.newAccount(ENGINE_KEY);
    executionEngine = blockchain.addExecutionEngine(p -> true, ENGINE_KEY);

    byte[] initRpc = OffChainFetchExchangeRate.initialize(engineAddress);
    contractAddress = blockchain.deployContract(sender, CONTRACT_BYTES, initRpc);

    OffChainFetchExchangeRate.State state = getState();
    Assertions.assertThat(state).isNotNull();
  }

  /**
   * The offchain will fetch exchange rates from the EU central bank and save the exchange rates in
   * the contract when calling {@link OffChainFetchExchangeRate#refreshExchangeRates}.
   */
  @ContractTest(previous = "deploy")
  void refreshExchangeRates() {
    addHttpResponse(200, EXAMPLE_EXCHANGE_RATES);
    signalExchangeRateRefresh();

    OffChainFetchExchangeRate.State state = getState();
    Map<String, Long> exchangeRates = state.exchangeRates();

    Assertions.assertThat(exchangeRates.get("DKK")).isEqualTo(746910);
  }

  /** Only connected execution engines can refresh exchange rates. */
  @ContractTest(previous = "deploy")
  void cannotRefreshExchangeRates() {
    byte[] rpc =
        OffChainFetchExchangeRate.updateExchangeRates(new ExchangeRates("", "", List.of()));
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(sender, contractAddress, rpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "Only the assigned engine, 008E97778954C30A5EBA91D99A862C3AE14C708898, may update"
                + " exchange rates");
  }

  /**
   * The offchain will not save anything to the contract if the response from the bank contains an
   * invalid status code.
   */
  @ContractTest(previous = "deploy")
  void errorResponse() {
    executionEngine.stop();
    addHttpResponse(500, EXAMPLE_EXCHANGE_RATES);
    signalExchangeRateRefresh();
    assertExecutionEnginePanicsWithMessage("Invalid status code from server: 500");
  }

  /** The offchain will not save anything to the contract if the response from the bank is empty. */
  @ContractTest(previous = "deploy")
  void emptyResponse() {
    executionEngine.stop();
    addHttpResponse(200, "");
    signalExchangeRateRefresh();
    assertExecutionEnginePanicsWithMessage("Timestamp must be present in input");
  }

  /**
   * The offchain will not save anything to the contract if the response from the bank is invalid
   * utf-8.
   */
  @ContractTest(previous = "deploy")
  void nonUtf8Response() {
    executionEngine.stop();
    addResponse(200, new byte[] {-128, -127, -126, -125});
    signalExchangeRateRefresh();
    assertExecutionEnginePanicsWithMessage("Response from server should be valid utf-8");
  }

  /**
   * The offchain will not save anything to the contract if the response from the bank has no
   * entries.
   */
  @ContractTest(previous = "deploy")
  void missingEntriesInResponse() {
    executionEngine.stop();
    addHttpResponse(200, EXAMPLE_EXCHANGE_RATES_WITHOUT_ENTRIES);
    signalExchangeRateRefresh();
    assertExecutionEnginePanicsWithMessage("Could not find any exchange rates");
  }

  /** Send refresh to contract and assert that the execution engine fails. */
  private void assertExecutionEnginePanicsWithMessage(String errorMessage) {
    Assertions.assertThatThrownBy(() -> executionEngine.resume())
        .hasStackTraceContaining(errorMessage);

    OffChainFetchExchangeRate.State state = getState();
    Map<String, Long> exchangeRates = state.exchangeRates();

    Assertions.assertThat(exchangeRates.size()).isEqualTo(0);
  }

  /** Send refresh request to contract. */
  private void signalExchangeRateRefresh() {
    byte[] rpc = OffChainFetchExchangeRate.refreshExchangeRates();
    blockchain.sendAction(engineAddress, contractAddress, rpc);
  }

  /** Add predefined response for exchange rate api endpoint. */
  private void addHttpResponse(int statusCode, String content) {
    addResponse(statusCode, content.getBytes(StandardCharsets.UTF_8));
  }

  /** Add predefined response for exchange rate api endpoint. */
  private void addResponse(int statusCode, byte[] content) {
    HttpResponseData response =
        new HttpResponseData(statusCode, Map.of(), Bytes.fromBytes(content));

    blockchain.addHttpResponse(
        "GET", "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml", response);
  }

  /** Get the state of the contract. */
  private OffChainFetchExchangeRate.State getState() {
    return OffChainFetchExchangeRate.State.deserialize(
        blockchain.getContractState(contractAddress));
  }
}
