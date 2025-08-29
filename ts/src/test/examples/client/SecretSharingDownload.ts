import { BlockchainAddress } from "@partisiablockchain/blockchain-api-transaction-client";
import BN from "bn.js";
import { SecretShareFactory } from "../secretsharing/SecretShareFactory";
import { SecretShares } from "../secretsharing/SecretShares";
import { getFactory, getSharingClient } from "./SecretSharingUpload";

/**
 * Minimal DEMO program for download secret-sharings to secret-sharing contract.
 *
 * <p>Uses a predefined secret-key, and blockchain reader node.
 *
 * <p>Argument format: <code><SHARING MODE> <CONTRACT ADDRESS> <ID: NUM></code>
 *
 * <p>Where <code><SHARING MODE></code> is either <code>xor</code> or <code>shamir</code>
 */
async function main() {
  if (process.argv.length !== 5) {
    console.error(
      `Invalid number of arguments. Expected '3' but received ${process.argv.length - 2}. Correct format is:\n` +
        "npm run download-shares <SHARING MODE> <CONTRACT ADDRESS> <SHARE ID>"
    );
    return;
  }
  const sharingMode = process.argv[2];
  const factory: SecretShareFactory<SecretShares> = getFactory(sharingMode);
  const contractAddress: BlockchainAddress = process.argv[3];
  const id: BN = new BN(process.argv[4]);

  const sharingClient = getSharingClient(contractAddress, factory);
  const secret = await sharingClient.downloadAndReconstruct(id);
  // eslint-disable-next-line no-console
  console.log(secret.toString("utf-8"));
}

if (require.main === module) {
  main();
}
