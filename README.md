# example-contracts
The Partisia Blockchain Foundation provides the following reviewed smart contracts,
as examples of real world problems with a blockchain solution.
The easiest way of getting started coding smart contracts is to learn from concrete examples
showing smart contracts solving problems similar to the one you need to solve.

This repository contains multiple example smart contracts as a virtual cargo workspace.
To compile all the contracts using the partisia-contract tool run:

    cargo partisia-contract build --release

To compile a single contract change directory to the specific contract and run the same command.
For example:

    cd token
    cargo partisia-contract build --release

The compiled wasm/zkwa and abi files are located in

    target/wasm32-unknown-unknown/release

To run the test suite, run the following command:
    
    ./run-java-tests.sh

To generate the code coverage report, run the following command:
    
    cargo partisia-contract build --coverage

The coverage report will be located in java-test/target/coverage