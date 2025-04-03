# DNS voting client

Example contract using [the DNS contract](../dns) to route invocations to
contracts by using the domain name of the wanted contract.

Shows how a smart contract can use the DNS to look up other smart contracts by their domain name, instead of by their address.
Here you can vote using a domain, where the address of a [voting contract](../voting) is fetched using the [DNS server](../dns).
The vote is then forwarded to that voting contract.

This works by instantiating the client with the DNS address, and invoking the vote action with the voting domain and vote as parameters.
