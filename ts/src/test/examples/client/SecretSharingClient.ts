import { SecretShareFactory } from "../secretsharing/SecretShareFactory";
import { ec } from "elliptic";
import {
  BlockchainAddress,
  BlockchainTransactionClient,
  Hash,
  SenderAuthenticationKeyPair,
  Signature,
} from "@partisiablockchain/blockchain-api-transaction-client";
import {
  NodeConfig,
  OffChainSecretSharing,
  registerSharing,
  requestDownload,
} from "../generated/OffChainSecretSharing";
import {
  BlockchainStateClientImpl,
  Hash as CodegenHash,
  BlockchainAddress as CodegenAddress,
} from "@partisiablockchain/abi-client";
import { SecretShares } from "../secretsharing/SecretShares";
import { randomBytes } from "crypto";
import BN from "bn.js";
import { CryptoUtils } from "../CryptoUtils";
import { BigEndianByteOutput } from "@secata-public/bitmanipulation-ts";
import KeyPair = ec.KeyPair;
import { getRequest, putRequest } from "../Api";

/**
 * Contract-specific client for interacting with a {@code off-chain-secret-sharing} contract.
 *
 * <p>This client implements required login for {@link #registerAndUploadSharing uploading} and
 * {@link #downloadAndReconstruct downloading} shares from the nodes of a given secret-sharing
 * contract.
 *
 * <p>Based on the java implementation of the secret-sharing client.
 */
export class SecretSharingClient<ShareT extends SecretShares> {
  static readonly GAS_COST_REGISTER_SHARING: number = 50_000;
  static readonly GAS_COST_REQUEST_DOWNLOAD: number = 50_000;
  private readonly transactionSender: BlockchainTransactionClient;
  private readonly offChainSecretSharingContractAddress: BlockchainAddress;
  private readonly senderKey: KeyPair;
  private readonly secretSharesFactory: SecretShareFactory<ShareT>;
  private readonly offChainSecretSharingContract: OffChainSecretSharing;

  /**
   * Create a new {@link SecretSharingClient}.
   *
   * @param transactionSender An transaction sender for interacting with the blockchain.
   * @param offChainSecretSharingContractAddress Address of the secret-sharing smart contract.
   * @param senderKey key pair to sign transactions using.
   * @param secretSharesFactory Factory for {@link SecretShares}. Used to work with shares.
   * @param offChainSecretSharingContract Client used to access the state of the smart contract.
   */
  private constructor(
    transactionSender: BlockchainTransactionClient,
    offChainSecretSharingContractAddress: BlockchainAddress,
    senderKey: KeyPair,
    secretSharesFactory: SecretShareFactory<ShareT>,
    offChainSecretSharingContract: OffChainSecretSharing
  ) {
    this.transactionSender = transactionSender;
    this.offChainSecretSharingContractAddress = offChainSecretSharingContractAddress;
    this.senderKey = senderKey;
    this.secretSharesFactory = secretSharesFactory;
    this.offChainSecretSharingContract = offChainSecretSharingContract;
  }

  /**
   * Create a new {@link SecretSharingClient}.
   *
   * @param baseReaderUrl URL of the Partisia Blockchain reader node.
   * @param offChainSecretSharingContractAddress Address of the secret-sharing smart contract.
   * @param senderKey key pair to sign transactions using.
   * @param secretSharesFactory Factory for {@link SecretShares}. Used to work with shares.
   */
  public static create<ShareT extends SecretShares>(
    baseReaderUrl: string,
    offChainSecretSharingContractAddress: BlockchainAddress,
    senderKey: KeyPair,
    secretSharesFactory: SecretShareFactory<ShareT>
  ): SecretSharingClient<ShareT> {
    return new SecretSharingClient<ShareT>(
      BlockchainTransactionClient.create(baseReaderUrl, new SenderAuthenticationKeyPair(senderKey)),
      offChainSecretSharingContractAddress,
      senderKey,
      secretSharesFactory,
      new OffChainSecretSharing(
        BlockchainStateClientImpl.create(baseReaderUrl),
        CodegenAddress.fromString(offChainSecretSharingContractAddress)
      )
    );
  }

  /**
   * Register and upload the given plain-text value as secret-shares to the contract's assigned
   * nodes.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @param sharingPlainText Plain text of the secret-sharing. Not nullable.
   * @throws Error if the sharing id is already in use.
   */
  public async registerAndUploadSharing(sharingId: BN, sharingPlainText: Buffer) {
    // eslint-disable-next-line no-console
    console.log("Creating shares for sharing %d", sharingId);
    const shares: ShareT = await this.createNoncedSharings(sharingPlainText);
    await this.registerSharing(sharingId, shares);
    await this.uploadShares(sharingId, shares);
  }

  /**
   * Download the specified sharing and reconstruct the plain-text value from the returned
   * secret-shares.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @returns Reconstructed plain-text. Not nullable.
   * @throws Error if the user doesn't have permission to access the given sharing.
   */
  public async downloadAndReconstruct(sharingId: BN): Promise<Buffer> {
    // eslint-disable-next-line no-console
    console.log("Request access to secret sharing with id %d", sharingId);
    await this.requestDownload(sharingId);

    const nodes = await this.getEngines();
    // eslint-disable-next-line no-console
    console.log("Downloading share with id %s from %s engines", sharingId, nodes.length);
    const promises: Array<Promise<Buffer | undefined>> = [];
    for (const node of nodes) {
      promises.push(this.downloadShareFromEngine(sharingId, node));
    }
    const shareBytes = await Promise.all(promises);
    // eslint-disable-next-line no-console
    console.log("All shares received");
    const expectedCommitments = await this.getExpectedCommitments(sharingId);
    const filteredShares = filterSharesFromCommitments(expectedCommitments, shareBytes);
    const shares = this.secretSharesFactory.fromSharesBytes(filteredShares);

    // eslint-disable-next-line no-console
    console.log("Reconstructing secret");
    return removeNoncePrefix(shares.reconstructPlainText());
  }

  /**
   * Create nonce-prefixed secret sharings.
   *
   * @param sharingPlainText The plain text of the secret.
   * @returns Secret shares.
   */
  private async createNoncedSharings(sharingPlainText: Buffer): Promise<ShareT> {
    const engines = await this.getEngines();
    return this.secretSharesFactory.fromPlainText(
      engines.length,
      prefixWithRandomNonce(sharingPlainText)
    );
  }

  /**
   * Register the sharing with the given identifier.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @param shares Shares to register.
   * @throws Error if the sharing id is already in use.
   */
  private async registerSharing(sharingId: BN, shares: ShareT) {
    // eslint-disable-next-line no-console
    console.log("Registering a new sharing with id %s to the contract", sharingId);
    const rpc = registerSharing(
      sharingId,
      getCommitments(shares).map((h) => CodegenHash.fromString(h))
    );
    const sent = await this.transactionSender.signAndSend(
      { address: this.offChainSecretSharingContractAddress, rpc },
      SecretSharingClient.GAS_COST_REGISTER_SHARING
    );
    const transactionTree = await this.transactionSender.waitForSpawnedEvents(sent);
    if (transactionTree.hasFailures()) {
      throw new Error("Unable to register sharing with id " + sharingId);
    }
  }

  /**
   * Send transaction to the contract requesting download of shares.
   *
   * @param sharingId Identifier for the secret sharing.
   * @throws Error if the user doesn't have permission to access the given sharing.
   */
  private async requestDownload(sharingId: BN) {
    const rpc = requestDownload(sharingId);
    const sent = await this.transactionSender.signAndSend(
      { address: this.offChainSecretSharingContractAddress, rpc },
      SecretSharingClient.GAS_COST_REQUEST_DOWNLOAD
    );
    const transactionTree = await this.transactionSender.waitForSpawnedEvents(sent);
    if (transactionTree.hasFailures()) {
      throw new Error("Unable to request download of shares with id " + sharingId);
    }
  }

  /**
   * Upload the given plain-text value as secret-shares to the contract's assigned nodes.
   *
   * @param sharingId Identifier for the secret sharing. Not nullable.
   * @param shares Shares to upload.
   */
  private async uploadShares(sharingId: BN, shares: ShareT) {
    const engines = await this.getEngines();
    // eslint-disable-next-line no-console
    console.log("Secret sharing and uploading to id %s for %s engines", sharingId, engines.length);
    const promises = [];
    for (let i = 0; i < engines.length; i++) {
      promises.push(this.uploadShareToEngine(sharingId, engines[i], shares.getShareBytes(i)));
    }
    await Promise.all(promises);
    // eslint-disable-next-line no-console
    console.log("Successfully uploaded share with id %s", sharingId);
  }

  /**
   * Upload a share to a specific node.
   *
   * @param sharingId the identifier of the sharing to upload
   * @param node node specific information
   * @param shareData the share bytes to upload
   * @throws Error if unable to upload the share
   */
  private async uploadShareToEngine(sharingId: BN, node: NodeConfig, shareData: Buffer) {
    const timestamp = new Date().getTime();
    const signature = createSignatureForOffChain(
      this.senderKey,
      node.address.asString(),
      this.offChainSecretSharingContractAddress,
      "PUT",
      sharingId,
      timestamp,
      shareData
    );
    // eslint-disable-next-line no-console
    console.log(
      "Uploading share %s with id %s to engine %s",
      shareData.toString("hex"),
      sharingId,
      node.address.asString()
    );
    const url = buildUrlForSharing(this.offChainSecretSharingContractAddress, node, sharingId);
    const ok = await sendWithRetries(
      () => putRequest(url, shareData, authorizationHeader(signature, timestamp)),
      (b) => b
    );
    if (!ok) {
      throw new Error(`Unable to upload share to engine ${node.address}`);
    }
  }

  /**
   * Download a share from specific engine.
   *
   * @param sharingId identifier of the sharing to download
   * @param node node specific information
   * @returns the downloaded share, or undefined if it couldn't download the share
   */
  private async downloadShareFromEngine(
    sharingId: BN,
    node: NodeConfig
  ): Promise<Buffer | undefined> {
    const timestamp = new Date().getTime();
    const signature = createSignatureForOffChain(
      this.senderKey,
      node.address.asString(),
      this.offChainSecretSharingContractAddress,
      "GET",
      sharingId,
      timestamp,
      Buffer.from([])
    );
    const url = buildUrlForSharing(this.offChainSecretSharingContractAddress, node, sharingId);
    const receivedShare = await sendWithRetries(
      () => getRequest(url, authorizationHeader(signature, timestamp)),
      (p) => p != undefined
    );
    if (receivedShare != undefined) {
      // eslint-disable-next-line no-console
      console.log(
        "Received share %s from engine: %s",
        receivedShare.toString("hex"),
        node.address.asString()
      );
    } else {
      // eslint-disable-next-line no-console
      console.log("Unable to receive share from %s", node.address.asString());
    }
    return receivedShare;
  }

  /**
   * Get the list of the node engines assigned to the contract.
   *
   * @return List of assigned node engines.
   */
  private async getEngines(): Promise<NodeConfig[]> {
    const state = await this.offChainSecretSharingContract.getState();
    return state.nodes;
  }

  /**
   * Get the sharings that have been registered on-chain. Can be used to validate whether the nodes
   * returned the correct shares.
   *
   * @param sharingId Identifier of the secret-sharing to get commitments for.
   * @return The share commitments.
   */
  private async getExpectedCommitments(sharingId: BN): Promise<Hash[]> {
    const state = await this.offChainSecretSharingContract.getState();
    const sharing = await state.secretSharings.get(sharingId);
    if (sharing == undefined) {
      throw new Error("Unable to get sharing with id " + sharingId);
    }
    return sharing.shareCommitments.map((h) => h.asString());
  }
}

/**
 * Validates that all engines have returned the correct shares. If they haven't, it might indicate a
 * malicious party.
 *
 * @param expectedCommitments the expected commitments from the chain
 * @param shares Raw shares to validate.
 * @throws Error if any of the shares doesn't match the expected commitments.
 */
export function filterSharesFromCommitments(
  expectedCommitments: Hash[],
  shares: Array<Buffer | undefined>
): Array<Buffer | undefined> {
  const filteredShares: Array<Buffer | undefined> = [];
  for (let i = 0; i < shares.length; i++) {
    const share = shares[i];
    if (share == undefined) {
      filteredShares.push(undefined);
      continue;
    }
    const commitment = CryptoUtils.hashBuffer(share).toString("hex");
    if (commitment === expectedCommitments[i]) {
      filteredShares.push(share);
    } else {
      console.warn(
        `Engine number ${i} did not return the correct secret-share.\n` +
          `On-chain commitment: ${expectedCommitments[i]}\nReceived share's hash: ${commitment}`
      );
      filteredShares.push(undefined);
    }
  }
  return filteredShares;
}

const NONCE_LENGTH = 32;

/**
 * Create a share with a 32-byte nonce prefix and the real data.
 *
 * <p>Inverse of {@link #removeNoncePrefix}.
 *
 * @param plainText Data to prefix with nonce.
 * @returns Prefixed data.
 */
function prefixWithRandomNonce(plainText: Buffer): Buffer {
  const nonce = randomBytes(NONCE_LENGTH);
  return Buffer.concat([nonce, plainText]);
}

/**
 * Remove nonce prefix.
 *
 * <p>Inverse of {@link #prefixWithRandomNonce}.
 *
 * @param prefixedData Nonce-prefixed data.
 * @returns Data without the nonce-prefix
 */
function removeNoncePrefix(prefixedData: Buffer): Buffer {
  return prefixedData.subarray(NONCE_LENGTH);
}

/**
 * Create the signature for authorization of the http requests against the off chain secret
 * share code.
 *
 * @param senderKey Key to sign signature with.
 * @param engineAddress Address of the node to send the request to.
 * @param contractAddress Address of the contract to send the request to.
 * @param method http method (GET or PUT)
 * @param sharingId Identifier of the secret-sharing.
 * @param timestamp The time when signed
 * @param data the secret to send
 * @returns the created signature.
 */
function createSignatureForOffChain(
  senderKey: KeyPair,
  engineAddress: BlockchainAddress,
  contractAddress: BlockchainAddress,
  method: string,
  sharingId: BN,
  timestamp: number,
  data: Buffer
): Signature {
  const buffer = BigEndianByteOutput.serialize((stream) => {
    stream.writeBytes(Buffer.from(engineAddress, "hex"));
    stream.writeBytes(Buffer.from(contractAddress, "hex"));
    stream.writeString(method);
    stream.writeString(contractUri(sharingId));
    stream.writeI64(new BN(timestamp));
    stream.writeI32(data.length);
    stream.writeBytes(data);
  });
  const hash = CryptoUtils.hashBuffer(buffer);
  const signature = senderKey.sign(hash);
  return CryptoUtils.signatureToBuffer(signature).toString("hex");
}

/**
 * The smart contract's internal URI for a secret-sharing.
 *
 * @param sharingId Identifier of the secret sharing.
 * @returns URI to access or modify the secret-sharing.
 */
function contractUri(sharingId: BN): string {
  return "/shares/" + sharingId.toString();
}

/**
 * Get the url for uploading or downloading shares to an engine.
 *
 * @param contractAddress address of the contract
 * @param node node specific information
 * @param sharingId identifier of the secret sharing
 */
function buildUrlForSharing(
  contractAddress: BlockchainAddress,
  node: NodeConfig,
  sharingId: BN
): string {
  return `${node.endpoint}/offchain/${contractAddress}${contractUri(sharingId)}`;
}

/**
 * Create Authorization header.
 *
 * @param signature the signature of the request
 * @param timestamp the time of the request
 * @returns the Authorization header.
 */
function authorizationHeader(signature: Signature, timestamp: number): HeadersInit {
  return {
    Authorization: `secp256k1 ${signature} ${timestamp}`,
  };
}

/**
 * Send a request multiple times until it succeeds the check or the maximum number of tries has been reached.
 *
 * @param request the request to retry
 * @param check predicate to check if the request succeeded
 * @returns the result of the succeeding request or the result of the latest request if none succeeded
 */
async function sendWithRetries<T>(
  request: () => Promise<T>,
  check: (received: T) => boolean
): Promise<T> {
  const numRetries = 10;
  let latestResult: T | undefined;
  for (let i = 0; i < numRetries; i++) {
    latestResult = await request();
    if (check(latestResult)) {
      return latestResult;
    }
    await new Promise((f) => setTimeout(f, 200));
  }
  return latestResult as T;
}

function getCommitments(share: SecretShares): Hash[] {
  return Array.from({ length: share.numShares() }, (_value, key) => {
    const shareBytes = share.getShareBytes(key);
    return CryptoUtils.hashBuffer(shareBytes).toString("hex");
  });
}
