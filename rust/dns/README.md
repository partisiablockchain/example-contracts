# DNS

Example contract implementing a simplified DNS, which allows users to name
individual contract addresses, much like the world-wide DNS system names IP
addresses.

## Usage

The DNS contract works by registering a domain to a given address, which can then be looked up by others.
Only the person registering a domain can remove it and update its corresponding address.

The DNS allows for registering, changing, removing and looking up a given domain.
Registering a domain will fail if the domain is taken, while the latter three will fail if the domain is not already registered.

Compile the dns contract by running the following command.
````shell
    cargo pbc build --release
````

For the following commands, you will need to specify your private key.
This can be done using either the option ````--pk=<private-key>```` or the command ````cargo pbc config privatekey <path-to-private-key>````.

Deploy the dns contract with the following command.
````shell
    cargo pbc transaction deploy ../target/wasm32-unknown-unknown/release/dns.wasm ../target/wasm32-unknown-unknown/release/dns.abi
````

Register a domain on the DNS, by running the following command.
````shell
    cargo pbc transaction action <dns-contract-address> register_domain <domain> <domain-address>
````

## Using the DNS to lookup the address of a voting contract.

The [dns-voting-client](../dns-voting-client/README.md) contract implements a client that uses the DNS to vote on a contract for a given domain.
To use the DNS voting client, compile the dns-voting-client contract with the following command.
````shell
    cargo pbc build --release
````

Deploy the DNS voting client with your chosen voting domain, using the following command.
````shell
    cargo pbc transaction deploy dns_voting_client.wasm dns_voting_client.abi
````

All wasm and abi files can be found inside [../target/wasm32-unknown-unknown/release](../target/wasm32-unknown-unknown/release).

Deploy [the voting contract](../voting/README.md), with the voting client as an eligible voter, with the following command.
````shell
    cargo pbc transaction deploy voting.wasm voting.abi <proposal-id> [ <dns-voting-client-contract-address> ] <voting-deadline>
````

Register the domain for voting in the DNS.
````shell
    cargo pbc transaction action <dns-contract-address> register_domain <voting-domain> <voting-address>
````

Vote using the voting client with the following command.
````shell
    cargo pbc transaction action <dns-voting-client-contract-address> vote <voting-domain> <vote>
````
