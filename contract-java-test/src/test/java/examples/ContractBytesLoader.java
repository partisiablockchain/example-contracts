package examples;

import com.partisiablockchain.language.junit.ContractBytes;
import java.nio.file.Path;

/** Smart contract bytecode loader. */
public final class ContractBytesLoader {

  private ContractBytesLoader() {}

  /** Folder containing compiled smart contracts. */
  private static final Path CONTRACTS_FOLDER =
      Path.of("../rust/target/wasm32-unknown-unknown/release");

  /**
   * Load the smart contract with the given project name.
   *
   * @param contractName Project name of the smart contract. Not nullable.
   * @return Loaded smart contract.
   */
  public static ContractBytes forContract(String contractName) {
    return ContractBytes.fromPbcFile(
        CONTRACTS_FOLDER.resolve(contractName + ".pbc"),
        CONTRACTS_FOLDER.resolve(contractName + "_runner"));
  }
}
