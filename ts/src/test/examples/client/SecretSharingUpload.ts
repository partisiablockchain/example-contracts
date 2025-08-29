import { BlockchainAddress } from "@partisiablockchain/blockchain-api-transaction-client";
import BN from "bn.js";
import { SecretSharingClient } from "./SecretSharingClient";
import { CryptoUtils } from "../CryptoUtils";
import { XorSecretShares } from "../secretsharing/XorSecretShares";
import { ShamirSecretShares } from "../secretsharing/ShamirSecretShares";
import { SecretShareFactory } from "../secretsharing/SecretShareFactory";
import { SecretShares } from "../secretsharing/SecretShares";

/**
 * Minimal DEMO program for uploading secret-sharings to secret-sharing contract.
 *
 * <p>Uses a predefined secret-key, and blockchain reader node.
 *
 * <p>Argument format: <code><SHARING MODE> <CONTRACT ADDRESS> <ID: NUM> <SECRET: STR></code>
 *
 * <p>Where <code><SHARING MODE></code> is either <code>"xor"</code> or <code>"shamir"</code>
 */
async function main() {
  if (process.argv.length !== 6) {
    console.error(
      `Invalid number of arguments. Expected '4' but received ${process.argv.length - 2}. Correct format is:\n` +
        "npm run upload-shares <SHARING MODE> <CONTRACT ADDRESS> <SHARE ID> <PLAINTEXT>"
    );
    return;
  }
  const sharingMode = process.argv[2];
  const factory: SecretShareFactory<SecretShares> = getFactory(sharingMode);
  const contractAddress: BlockchainAddress = process.argv[3];
  const id: BN = new BN(process.argv[4]);
  const secret: Buffer = Buffer.from(process.argv[5], "utf-8");

  const sharingClient = getSharingClient(contractAddress, factory);
  await sharingClient.registerAndUploadSharing(id, secret);
}

/**
 * Get the corresponding factory according to the supplied sharing mode.
 *
 * @param sharingMode "xor" for xor secret sharing or "shamir" for shamir secret sharing
 * @returns the secret sharing factory
 * @throws Error if the supplied sharing mode is neither "xor" nor "shamir"
 */
export function getFactory(sharingMode: string): SecretShareFactory<SecretShares> {
  if (sharingMode === "xor") {
    return XorSecretShares.FACTORY;
  } else if (sharingMode === "shamir") {
    return ShamirSecretShares.FACTORY;
  } else {
    throw new Error("Invalid secret sharing mode. Valid modes are [xor, shamir].");
  }
}

/**
 * Create new {@link SecretSharingClient} for the specific contract.
 *
 * @param contractAddress Address of the contract to interact with.
 * @param factory factory for creating secret shares
 * @return Client for a secret-sharing smart-contract.
 */
export function getSharingClient<T extends SecretShares>(
  contractAddress: BlockchainAddress,
  factory: SecretShareFactory<T>
): SecretSharingClient<T> {
  const readerUrl = "https://node1.testnet.partisiablockchain.com";
  const keyPair = CryptoUtils.privateKeyToKeypair("aa");
  return SecretSharingClient.create(readerUrl, contractAddress, keyPair, factory);
}

if (require.main === module) {
  main();
}
