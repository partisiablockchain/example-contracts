# Proxy Contract

This is a proof-of-concept "proxy" smart contract.
It forwards an authorized user's transaction to other smart contracts.

It does this by having an off-chain component act as a REST-based API server that will send the user's transaction requests on their behalf.

Authorization is implemented through bearer tokens.
If a user sends a correct bearer token in their HTTP request to the off-chain component then they are considered authorized.

This smart contract is aimed to appeal to users who are comfortable with token authorization but not cryptographic signatures.
This is important because all on-chain transactions require a cryptographic signature.
With this smart contract, users can use token authorization instead of cryptographic signatures by proxying their transactions through an off-chain component.

Finally, it should be noted that since the user interacts solely through HTTP(S) requests, it allows them to interact with the on-chain without needing any gas.

## Design

There are three parts of the proxy smart contract:

1. **Initialization**

   During which the SHA-256 hashes of valid authorization tokens are specified.

2. **Off-chain component**

    Implements token authorization and forwards users' transactions, cryptographically signed by the engine, to the on-chain component.

3. **On-chain component**

    Forwards incoming transactions to their final destination if they come from an engine assigned to this contract.

### Initialization

Upon initialization of the smart contract, a set of hashes of valid authorization tokens are provided to the smart contract.
If a user possesses a token whose hash is in that set, they are considered authorized.

### Off-chain component

The off-chain component accepts HTTP(S) requests that should contain:

- A bearer token
- A serialized `TransactionRequest` struct that contains the target smart contract, serialized transaction payload, and total gas cost for all computation involved in proxying

The token is used for authorization by the off-chain component.
This is done by checking if its hash is in the set of hashes stored on-chain.

If authorization is successful, the execution engine constructs a `TransactionPayload` struct and forwards it to the on-chain component with the gas amount specified in `TransactionRequest`.

### On-chain component

The on-chain component deserializes the `TransactionPayload` struct and extracts (1) a target smart contract and (2) a payload that is the serialization of the transaction the user wants to send to the target smart contract.
Then it forwards the payload to the target smart contract.

## HTTP(S) Request

For this contract, the HTTP(S) request should be a POST request and MUST have the following HTTP header:

```html
Authorization: Bearer <TOKEN>
```

The HTTP(S) request body should contain a `TransactionRequest` serialized in big-endian form.
A `TransactionRequest` has the following form:

```rust,ignore
pub struct TransactionRequest {
    /// The target smart contract address to forward the payload to.
    target_smart_contract: Address,
    /// The serialised transaction to be forwarded.
    payload: Vec<u8>,
    /// The total gas needed for the transaction.
    gas: u64,
}
```

Note that the execution engine requires gas when sending a `TransactionPayload` to the on-chain.
The amount of gas it will attempt to use is given by the user specified struct field `gas`.

## Example HTTP Request

As an example, let's send an HTTP request that submits a vote to the [Voting Example Contract](../voting/README.md) via this proxy contract.

One can make invocations towards the voting contract via the proxy contract by using the provided [upload-proxy-vote](../../scripts/upload-proxy-vote.sh) script
(we assume the voting and proxy contracts have been deployed and that execution engines have been [set up](https://partisiablockchain.gitlab.io/documentation/node-operations/run-an-execution-engine.html)).

The script expects four arguments:

1. `true/false` represents whether the vote we are proxying should be for `true` or `false`.
2. `<AUTHORIZATION_TOKEN>` is the authorization token to use when trying to authorize with the off-chain.
3. `<VOTE_CONTRACT_ADDRESS>` is the blockchain address, as a hexadecimal string, of the voting contract we are trying to target.
4. `<PROXY_ENDPOINT>` is a full [endpoint](https://partisiablockchain.gitlab.io/documentation/smart-contracts/integration/using-off-chain-components-in-an-application.html) of an execution engine for the proxy contract.

We can check if the proxy is successful by looking at the browser.
The proxy contract should now have a new transaction with the "Forward" action.
Specifically, it should have an interaction that represents the execution engine sending the HTTP request and another interaction that represents the on-chain forwarding the payload to the voting contract.
Likewise, the voting contract should now have a new vote for "true".

## Security and Further Considerations

### HTTPS

The contract assumes that the HTTP connection between the user and the execution engine is secured through TLS or other mechanisms. It is the execution engine maintainer's job to setup HTTPS.

### Loss of Transaction Context

Original transaction context information is always lost with this smart contract.

For example, the `vote`  action in `Voting` checks the transaction context to determine who the sender was of the vote.

With the proxy smart contract, the `sender` will always be the *proxy smart contract*, not the original user.

Therefore, any user must be aware of the limitations of sending transactions through a proxy since information in the original transaction context is lost.
