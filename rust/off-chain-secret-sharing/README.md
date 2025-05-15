# Off-Chain Secret-Sharing Contract

Smart contract which allows users to store [secret shares](https://en.wikipedia.org/wiki/Secret_sharing).

Users register their intention of storing a secret share on chain, whereafter
they can upload the shares to the off-chain components configured in the state.

The uploading user can then later on download their shares again.

This is an example of how to use the **Self-Hosted Execution Engine**
system. Contract owner is responsible for running a self-hosted Execution
Engine which provides the storage solution.

## Example Client

This contract has an [example
client](../../contract-java-test/src/test/java/examples/client/SecretSharingClient.java)
that demonstrates how to interact with it from a Java code-base. Two bash
scripts have been created for conveniently testing this client. The
[`upload-shares.sh`](../../scripts/upload-shares.sh) script allows you to
secret share a variable to the given contract:

> [!tip]
> These scripts must be called from repository root.

```bash
bash scripts/upload-shares.sh $CONTRACT_ADDRESS 123 "Plaintext_of_Secret_Value"
```

Whereas the [`download-shares.sh`](../../scripts/download-shares.sh) script allows you to download and
reconstruct the plaintext of a variable from the same contract:

```bash
bash scripts/upload-shares.sh $CONTRACT_ADDRESS 123
```

## Example Usage, HTTP

The following flow shows how to use the off-chain secret-sharing contract as
a local test. It requires that you have setup an Execution Engine for the
blockchain you want to target:

1. [Deploy the off-chain secret-sharing
   contract](https://partisiablockchain.gitlab.io/documentation/smart-contracts/compile-and-deploy-contracts.html)
   with one Execution Engine:

```bash
cargo pbc transaction deploy --gas 10000000 ./target/wasm32-unknown-unknown/release/off_chain_secret_sharing.pbc \[ \{ "$EE_ADDRESS" "$EE_ENDPOINT" \} \]
```

2. Contract should now deployed and visible in the browser.
3. [Send an
   invocation](https://partisiablockchain.gitlab.io/documentation/smart-contracts/smart-contract-interactions-on-the-blockchain.html)
   that registers your intention to upload a Secret Sharing. Assign it some id
   (123 in this case.)

```bash
cargo pbc transaction action --gas 10000000 "$CONTRACT_ADDRESS" register_sharing 123
```

4. Contract state should now contain a `Sharing` with the id 123.
5. You can now upload your share to the assigned execution engines. See the
   [Authentication section below](#authentication) for how to create the
   signature.

```bash
curl -X PUT -d @SHARE_FILE_1 -H "Authorization: secp256k1 $SIGNATURE" "$EE_ENDPOINT/executioncontainer/$CONTRACT_ADDRESS/shares/123"
```

6. The sharing's structure in the contract state should now indicate that one
   of the engines has received the shares.
7. You can now download the the share again, from the assigned execution
   engine. See the [Authentication section below](#authentication) for how to
   create the signature.

```bash
curl -H "Authorization: secp256k1 $SIGNATURE" "$EE_ENDPOINT/executioncontainer/$CONTRACT_ADDRESS/shares/123"
```

## Authentication

> [!warning]
> This authentication protocol does not support timestamps. If a `GET` request is
> leaked it can be used to access secret shares forever.

> [!tip]
> You can modify the `Sharing::is_authenticated` method to use your own
> authentication system. You can also turn off authentication by replacing the
> `Sharing::is_authenticated` function body with one that returns `true` no
> matter what. This might be useful when experimenting.

This contract includes built-in authentication for all HTTP requests, which
ensures that only the owner of a secret-shared variable is capable of
downloading and reconstructing it.

The authentication protocol is based upon setting the `Authorization` header to
a value of the form `secp256k1 [SIGNATURE]`, where SIGNATURE is a hex-encoding
of a [SECP256k1 signature](https://en.bitcoin.it/wiki/Secp256k1) of a specific
message described in detail below. This authentication protocol was chosen
because it allows the smart contract to uniquely identify a user using the same
identity both on-chain and off-chain. Users and clients only need to keep track
of a single secret-key to be able to sign both on-chain transactions, and
off-chain HTTP requests.

### Auditable Downloads

User must authenticate with the smart contract before they are allowed to
download their shares from the engines. This improves auditability, and allows
auditors to create a full trace of when the secret sharing was created and
accessed.

> [!tip]
> This functionality can be disabled by removing the
> `assert_download_deadline_not_passed` function.

### Signature

The signature is computed by hashing and signing a message of the following
format. All values are serialized using the [RPC serialization
format](https://partisiablockchain.gitlab.io/documentation/smart-contracts/smart-contract-binary-formats.html#rpc-binary-format)
(E.g. serializing integers as big-endian):

- Execution engine address (21 bytes)
- Contract address (21 bytes)
- Request Method (`GET` or `PUT`), size prefixed (4+ bytes)
- Request URI (`/shares/{sharingId}`), size prefixed (4+ bytes)
- Request Body, size prefixed (4+ bytes)

### Signature Example

An example of how to sign a request for the shares `123213`:

```text
    GET /shares/123123
    Headers:
    - Authorization: secp256k1 [SIGNATURE]
```

Where `SIGNATURE` is computed as:

```text
Sign(Sha256(
    engineAddress ||
    contractAddress ||
    0x00000003 || "GET" ||
    0x0000000e || "/shares/123123" ||
    0x00000000 || ""))
```

Where `||` represents byte concatenation, and `"TEXT"` represents the given
text as ascii-encoded bytes, without a size prefix.
