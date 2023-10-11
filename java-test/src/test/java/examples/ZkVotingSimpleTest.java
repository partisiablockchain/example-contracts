package examples;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.ZkVotingSimple;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;
import com.secata.stream.BitOutput;
import com.secata.stream.CompactBitArray;
import java.nio.file.Path;
import org.assertj.core.api.Assertions;

/** Test the Zero Knowledge Simple Voting Contract. */
public final class ZkVotingSimpleTest extends JunitContractTest {

  private static final ContractBytes VOTING_SIMPLE_BYTES =
      ContractBytes.fromPaths(
          Path.of("../target/wasm32-unknown-unknown/release/zk_voting_simple.zkwa"),
          Path.of("../target/wasm32-unknown-unknown/release/zk_voting_simple.abi"),
          Path.of("../target/wasm32-unknown-unknown/release/zk_voting_simple_contract_runner"));

  private BlockchainAddress account1;
  private BlockchainAddress account2;
  private BlockchainAddress account3;
  private BlockchainAddress account4;
  private BlockchainAddress account5;
  private BlockchainAddress account6;

  private BlockchainAddress votingSimple;

  @ContractTest
  void deploy() {
    account1 = blockchain.newAccount(2);
    account2 = blockchain.newAccount(3);
    account3 = blockchain.newAccount(4);
    account4 = blockchain.newAccount(5);
    account5 = blockchain.newAccount(6);
    account6 = blockchain.newAccount(7);

    byte[] initRpc = ZkVotingSimple.initialize(10000);

    votingSimple = blockchain.deployZkContract(account1, VOTING_SIMPLE_BYTES, initRpc);

    byte[] contractByteState = blockchain.getContractState(votingSimple);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(contractByteState);

    Assertions.assertThat(state).isNotNull();
  }

  @ContractTest(previous = "deploy")
  void voteForAll() {
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account2, createSecretIntInput(0), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account3, createSecretIntInput(0), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account4, createSecretIntInput(0), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account5, createSecretIntInput(0), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account6, createSecretIntInput(0), new byte[] {0x40});

    blockchain.waitForBlockProductionTime(10500);

    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();

    blockchain.sendAction(account1, votingSimple, startVoteCount);

    byte[] contractByteState = blockchain.getContractState(votingSimple);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(contractByteState);

    Assertions.assertThat(state.voteResult()).isEqualTo(new ZkVotingSimple.VoteResult(1, 5, false));
  }

  @ContractTest(previous = "deploy")
  void voteYesForAll() {
    blockchain.sendSecretInput(votingSimple, account1, createSecretIntInput(1), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account2, createSecretIntInput(1), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account3, createSecretIntInput(1), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account4, createSecretIntInput(1), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account5, createSecretIntInput(1), new byte[] {0x40});
    blockchain.sendSecretInput(votingSimple, account6, createSecretIntInput(1), new byte[] {0x40});

    blockchain.waitForBlockProductionTime(10500);

    byte[] startVoteCount = ZkVotingSimple.startVoteCounting();

    blockchain.sendAction(account1, votingSimple, startVoteCount);

    byte[] contractByteState = blockchain.getContractState(votingSimple);

    ZkVotingSimple.ContractState state =
        ZkVotingSimple.ContractState.deserialize(contractByteState);

    Assertions.assertThat(state.voteResult()).isEqualTo(new ZkVotingSimple.VoteResult(6, 0, true));
  }

  CompactBitArray createSecretIntInput(int secret) {
    return BitOutput.serializeBits(output -> output.writeSignedInt(secret, 32));
  }
}
