# Zero Knowledge voting simple

ZK voting does the same as the public vote, with a notable exception the votes are anonymous and only the final result is revealed in the public state

Secret voting is a common Zero-knowledge MPC example, wherein several persons are interested in
voting upon some question, without revealing their personal preference, similar to many
democratic election processes.

This contract's flow follows as:

1. Initialization of contract with voting information, including owner and vote duration
2. Voters send their votes. (0 is against, any other value is for).
3. After the deadline, the vote counting can be started by anyone.
4. Zk Computation sums yes votes and no votes, and output each as a separate variable.
5. When computation is complete the contract will open the output variables.
6. The contract computes whether the vote was accepted or rejected.