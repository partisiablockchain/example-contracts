# Off-Chain Secret Sharing

Contract that allows user to store [secret shares](https://en.wikipedia.org/wiki/Secret_sharing).

Users register their intention of storing a secret share on chain, whereafter
they can upload the shares to the off-chain components configured in the state.

The uploading user can then later on download their shares again.

This contract is an example of how to use the **Self-Hosted Execution Engine**
system. Contract owner is responsible for running a self-hosted Execution
Engine which provides the storage solution.

## Authorization

> [!warning]
> This authentication protocol does not support timestamps. If a GET request is
> leaked it can be used to access secret shares forever.

Authentication is handled by having the owner of a share sign their http request when
communicating with the engines.

Each sharing has an owner, set to the sender of the `register_sharing` invocation. An
http request is only valid if there exists a valid signature signed by the owner of the
sharing.

The message to be signed consists of the following all serialized as rpc.

- Execution engine address
- Contract address
- Request method ("GET" or "PUT")
- Request Uri ("/shares/{sharingId}")
- Request body

The signature is places in the authorization header as a hex encoded string prefixed with `secp256k1 `.

The signature scheme used is ECDSA over the secp256k1 curve.

### Example

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

Where `||` represents byte concatenation.
