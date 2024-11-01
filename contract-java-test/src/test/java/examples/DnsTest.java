package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.Dns;
import com.partisiablockchain.language.codegenlib.AvlTreeMap;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.partisiablockchain.language.junit.exceptions.ActionFailureException;
import java.nio.file.Path;
import java.util.Locale;
import org.assertj.core.api.Assertions;

/** Test suite for the DNS contract. */
public final class DnsTest extends JunitContractTest {

  private static final ContractBytes DNS_CONTRACT_BYTES =
      ContractBytes.fromPaths(
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns.wasm"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns.abi"),
          Path.of("../rust/target/wasm32-unknown-unknown/release/dns_runner"));

  private BlockchainAddress voter;
  private BlockchainAddress admin;
  private BlockchainAddress dnsAddress;
  private BlockchainAddress testAddress1;
  private BlockchainAddress testAddress2;

  private Dns dnsContract;

  /**
   * Setup for all the other tests. Instantiates two accounts and deploys a DNS contract, and two
   * test contracts.
   */
  @ContractTest
  void setUp() {
    voter = blockchain.newAccount(2);
    admin = blockchain.newAccount(3);

    byte[] initDnsRpc = Dns.initialize();
    dnsAddress = blockchain.deployContract(voter, DNS_CONTRACT_BYTES, initDnsRpc);
    dnsContract = new Dns(getStateClient(), dnsAddress);

    testAddress1 = BlockchainAddress.fromString("0002131a2b3c6741b42cfa4c33a2830602a3f2e9ff");
    testAddress2 = BlockchainAddress.fromString("0002131a2b3c6741b42cfa4c33a2830602a3f2e9ff");
  }

  /** Users can register a domain in the DNS. */
  @ContractTest(previous = "setUp")
  public void register() {
    byte[] registerRpc = Dns.registerDomain("domainname", testAddress1);
    blockchain.sendAction(admin, dnsAddress, registerRpc);

    Dns.DnsState state = dnsContract.getState();
    AvlTreeMap<String, Dns.DnsEntry> records = state.records();

    Assertions.assertThat(records.get("domainname").address()).isEqualTo(testAddress1);
  }

  /** A user cannot register a domain, that is already registered. */
  @ContractTest(previous = "setUp")
  public void cannotRegisterTwice() {
    byte[] register1Rpc = Dns.registerDomain("domainname", testAddress1);
    blockchain.sendAction(admin, dnsAddress, register1Rpc);

    byte[] register2Rpc = Dns.registerDomain("domainname", testAddress2);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(admin, dnsAddress, register2Rpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Domain already registered");

    Dns.DnsState state = dnsContract.getState();
    AvlTreeMap<String, Dns.DnsEntry> records = state.records();

    Assertions.assertThat(records.size()).isEqualTo(1);
  }

  /** The owner of a domain can remove the domain. */
  @ContractTest(previous = "setUp")
  public void remove() {
    byte[] registerRpc = Dns.registerDomain("domainname", testAddress1);
    blockchain.sendAction(admin, dnsAddress, registerRpc);

    byte[] removeRpc = Dns.removeDomain("domainname");
    blockchain.sendAction(admin, dnsAddress, removeRpc);

    Dns.DnsState state = dnsContract.getState();
    AvlTreeMap<String, Dns.DnsEntry> records = state.records();

    Assertions.assertThat(records.size()).isEqualTo(0);
  }

  /** A user cannot remove a domain that has not been registered. */
  @ContractTest(previous = "setUp")
  public void removeNotExisting() {
    byte[] removeRpc = Dns.removeDomain("domainname");

    Assertions.assertThatThrownBy(() -> blockchain.sendAction(admin, dnsAddress, removeRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Could not find domain");
  }

  /** A user cannot remove a domain, that the user does not own. */
  @ContractTest(previous = "setUp")
  public void removeByNonOwner() {
    byte[] registerRpc = Dns.registerDomain("domainname", testAddress1);
    blockchain.sendAction(admin, dnsAddress, registerRpc);

    byte[] removeRpc = Dns.removeDomain("domainname");

    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter, dnsAddress, removeRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "Only the owner of the domain can delete it. Owner:"
                + " 00CECA763AFC2C2E6933DB580D4F788E4DF7D6E8E6, Sender:"
                + " 00B2E734B5D8DA089318D0D2B076C19F59C450855A");

    Dns.DnsState state = dnsContract.getState();
    AvlTreeMap<String, Dns.DnsEntry> records = state.records();

    Assertions.assertThat(records.size()).isEqualTo(1);
  }

  /** The owner can update the address linked to their domain. */
  @ContractTest(previous = "setUp")
  public void updateDomain() {
    byte[] registerRpc = Dns.registerDomain("domainname", testAddress1);
    blockchain.sendAction(admin, dnsAddress, registerRpc);

    byte[] updateRpc = Dns.updateDomain("domainname", testAddress2);
    blockchain.sendAction(admin, dnsAddress, updateRpc);

    Dns.DnsState state = dnsContract.getState();
    AvlTreeMap<String, Dns.DnsEntry> records = state.records();

    Assertions.assertThat(records.get("domainname").address()).isEqualTo(testAddress2);
  }

  /** A user cannot update the address of domain that has not been registered. */
  @ContractTest(previous = "setUp")
  public void updateNonExisting() {
    byte[] updateRpc = Dns.updateDomain("domainname", testAddress2);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(admin, dnsAddress, updateRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining("Could not find domain");
  }

  /** A user that does not own the domain, cannot update the domain. */
  @ContractTest(previous = "setUp")
  public void updateByNonOwner() {
    byte[] registerRpc = Dns.registerDomain("domainname", testAddress1);
    blockchain.sendAction(admin, dnsAddress, registerRpc);

    byte[] updateRpc = Dns.updateDomain("domainname", testAddress2);
    Assertions.assertThatThrownBy(() -> blockchain.sendAction(voter, dnsAddress, updateRpc))
        .isInstanceOf(ActionFailureException.class)
        .hasMessageContaining(
            "Only the owner of the domain can modify it. Owner: "
                + admin.writeAsString().toUpperCase(Locale.getDefault())
                + ", Sender: "
                + voter.writeAsString().toUpperCase(Locale.getDefault()));

    Dns.DnsState state = dnsContract.getState();
    AvlTreeMap<String, Dns.DnsEntry> records = state.records();

    Assertions.assertThat(records.get("domainname").address()).isEqualTo(testAddress1);
  }
}
