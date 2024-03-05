# Access Control

Example smart contract showcasing an access control system.
The security is defined through a partial order. If an action requires a security level of `A`, then only users with security
level `B`, where `B` is greater than or equal to `A`, can perform that action.

The contract allows for lending out an object to users, but only if they have high enough privilege.
The state contains a map of user levels, as well as a description of the object and who currently holds it.


The access control system works over generically defined security levels.