
# Ping

An example contract implementing a simple ping functionality.

### Functionality
Users can utilize the `ping` functionality of this contract mainly for two things:
* Check for the existence of other contracts.
* Transfer gas to a destination contract.

### Usage
The `ping` function creates a single interaction with the contract at `destination`, 
with a given `cost`.

By specifying the minimum `cost`, the network fee, 
users can easily check if a `destination` contract exists.

The `cost` is kept by the `destination` contract, thus users can use the contract 
to transfer gas to the `destination` contract.
To transfer the maximum possible gas amount the `cost` can be set to `None`,

Additionally, users can utilize `ping_no_callback`, to transfer gas 
without checking whether `destination` exists.
