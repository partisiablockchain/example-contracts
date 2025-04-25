#!/usr/bin/env bash

# Script downloading shares from a off-chain-secret-sharing contract.
#
# Format: <CONTRACT ADDRESS> <SHARE ID>

set -e
cd contract-java-test
mvn compile test-compile exec:java -Dexec.classpathScope=test -Dexec.mainClass="examples.client.SecretSharingDownload" -Dexec.args="$1 $2"
