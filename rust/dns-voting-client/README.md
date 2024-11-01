
# DNS voting client

An example contract implementing a simple DNS client.
This contract shows how a smart contract can use the DNS to look up other smart contracts by their domain name, instead of by their address.
Here you can vote using a domain, where the address of a [voting contract](../voting/README.md) is fetched using the [DNS server](../dns/README.md). 
The vote is then forwarded to that voting contract.

This works by instantiating the client with the DNS address, and invoking the vote action with the voting domain and vote as parameters.

For how to use this, see [the dns contract](../dns/README.md).
