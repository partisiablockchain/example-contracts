#!/bin/bash

compile_contracts() {
  if [ "$coverage" = true ]; then
    cargo partisia-contract build --release --coverage
  else
    cargo partisia-contract build --release
  fi
}

run_java_tests() {
  if [ "$coverage" = true ]; then
    test_with_coverage
  else
    test_without_coverage
  fi
}

test_with_coverage() {
  pushd java-test 1> /dev/null || exit
  # Run contract tests
  mvn test -Dcoverage
  # Merge profraw
  rust-profdata merge -sparse ./target/coverage/profraw/*.profraw --output=target/coverage/java_test.profdata
  # Generate report
  find ../target/wasm32-unknown-unknown/release/ -type f -executable -print | \
  sed "s/^/--object /" | \
  xargs rust-cov show --ignore-filename-regex=".cargo\.*" --ignore-filename-regex="target\.*" \
     --instr-profile=target/coverage/java_test.profdata --Xdemangler=rustfilt --format="html" \
        --output-dir=target/coverage/html

  popd 1> /dev/null || exit
}

test_without_coverage() {
  # Run contract tests
  pushd java-test 1> /dev/null || exit
  mvn test
  popd 1> /dev/null || exit
}

help() {
	echo "usage: ./run-java-tests.sh [-b][-c][-h]"
	echo "-b    Build the contracts before running tests (if coverage is enabled also generates the instrumented executables)"
	echo "-c    Test with coverage enabled"
	echo "-h    Print this help message"
	exit 0
}

while getopts :bch flag
do
        case "${flag}" in
                b) build=true ;;
                c) coverage=true ;;
                h) help ;;
                *) echo "Invalid option: -$flag." && help ;;
        esac
done

if [ "$build" = true ]; then
  compile_contracts
fi

run_java_tests
