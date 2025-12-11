# Off-Chain MPC signing

Smart contract for creating ECDSA signatures over the secp256k1 curve in MPC where no single party knows the signing
key.

The signing key is generated in MPC once the contract is deployed and the corresponding public key is uploaded to the
contract.

A user can call the action `sign_message` on the contract with a message they wish to be signed. The assigned engines
will then perform an MPC protocol resulting in the signature of the message being placed in the state of the contract.

This contract works with three nodes and is secure as long as at most one node is malicious.

## Design

The MPC protocol can be split into three phases

1. Setup
2. Preprocessing
3. Signing

### Setup

The goal of the setup phase is for the engines to exchange keys with each other, and to generate the signing key secret
shares.

To achieve this, the engines first create an ephemeral secret key, and uploads the corresponding public key to the
contract. Each engine then performs an elliptic curve Diffie-Hellman exchange with each other engine to generate seeds
for Pseudo Random Generators (PRG). These PRGs are then used to generate the signing key share.

Finally, each engine computes the public key share from their signing key share and uploads it to the contract, where it
is opened and placed in the state.

The setup phase can be completed by having each engine send two transactions.

### Preprocessing

To make signing be able to run faster the contract and engines can do some preprocessing and store the results until a
signing request is made.

Multiple preprocessed values can be created at once which saves on the number of rounds of
communication that is required between the engines.

The contract is customizable for the amount of preprocessed material the contract should have ready for signing
as well as the batch size of the preprocessing.

Each preprocessing run can be completed by having each node send four transactions.

### Signing

Finally, once a signing request has been made a preprocessed value is used to create the signature.

Only two engines are required to create signature, but only if the preprocessed material has already been created.
If the signature is unable to be created with only two engines (because one of the engines behaved maliciously), the
contract waits until the third engine has responded, at which point the signature is guaranteed to be created.

Each signing request can be completed by having each engine send a single transaction.

## Secret sharing scheme

The secret sharing scheme of the contract is the 1-out-of-3 Replicated Secret-Sharing Scheme.

In this scheme a secret sharing [x] is defined as the three values (x3, x2), (x1, x3), and (x2, x1) where
x = x1 + x2 + x3.

I.e. each share is replicated between two parties in such a way that no single party knows all the shares, but any two
nodes can reconstruct the share.