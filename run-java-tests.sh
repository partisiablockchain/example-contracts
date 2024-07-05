#!/usr/bin/env bash

compile_contracts() {
  pushd rust 1> /dev/null || exit
  if [ "$coverage" = true ]; then
    cargo partisia-contract build --release --coverage
  else
    cargo partisia-contract build --release
  fi
  popd 1> /dev/null || exit
}

run_java_tests() {
  if [ "$coverage" = true ]; then
    test_with_coverage
  else
    test_without_coverage
  fi
}

merge_and_report() {
  pushd java 1> /dev/null || exit

  # Determine profraw files
  find ./target/coverage/profraw/ -type f -name '*.profraw' > ./target/coverage/all-profraw-files

  # Merge profraw
  rust-profdata merge -sparse --input-files=./target/coverage/all-profraw-files --output=target/coverage/java_test.profdata

  # Generate report
  find ../rust/target/wasm32-unknown-unknown/release/ -type f -executable -print |
    sed "s/^/--object /" |
    xargs rust-cov show --ignore-filename-regex=".cargo\.*" --ignore-filename-regex="target\.*" \
      --instr-profile=target/coverage/java_test.profdata --Xdemangler=rustfilt --format="html" \
      --output-dir=target/coverage/html
  popd 1> /dev/null || exit
}

test_with_coverage() {
  # Run contract tests
  pushd java 1> /dev/null || exit
  mvn test -Dcoverage
  popd 1> /dev/null || exit

  merge_and_report
}

test_without_coverage() {
  # Run contract tests
  pushd java 1> /dev/null || exit
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

while getopts :bch flag; do
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
