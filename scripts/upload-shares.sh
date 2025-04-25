#!/usr/bin/env bash
# Script uploading shares to a off-chain-secret-sharing contract.
#
# Format: <CONTRACT ADDRESS> <SHARE ID> <PLAINTEXT>

set -e
cd contract-java-test
mvn compile test-compile exec:java -Dexec.classpathScope=test -Dexec.mainClass="examples.client.SecretSharingUpload" -Dexec.args="$1 $2 $3"
