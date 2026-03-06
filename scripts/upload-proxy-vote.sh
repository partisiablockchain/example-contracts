#!/usr/bin/env bash
# Script uploading a vote transaction request to an execution engine for a proxy contract.
#
# Format: <true/false> <AUTHORIZATION_TOKEN> <VOTE_CONTRACT_ADDRESS> <PROXY_ENDPOINT>

set -e
cd contract-java-test
mvn compile test-compile exec:java -Dexec.classpathScope=test -Dexec.mainClass="examples.client.ProxyVoteUpload" -Dexec.args="'$1' '$2' '$3' '$4'"
