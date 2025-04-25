# Voting

Example smart contract implementing a simple majority open ballot vote for a proposal among a fixed list of eligible voters.

The public vote has:
- a proposal id,
- a list of accounts that can participate,
- a list of votes.

The state of the contract shows the result, participants and if the vote is finished.

## Usage

* The owner of the proposal deploys a Vote smart contract to the blockchain and initializes it.
* Eligible voters can cast their vote until the deadline.
* After the deadline passes anyone can initiate counting of the votes.

