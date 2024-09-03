#!/usr/bin/env bash

function run_test_json {
	pushd test-projects/$1
	rm -rf target
	RUSTFLAGS="-Z save-analysis" cargo +nightly check
	popd
	cargo run test-projects/$1/target/debug/deps/save-analysis/binary_thing-*.json > test-projects/$1/target/$1.stdout
	output=$(cat test-projects/$1/target/$1.stdout)
	expected=$(cat test-projects/$1/$1.stdout)
	if [ "$expected" != "$output" ]; then
		echo "Mismatch. Expected:"
		echo "---------------------"
		echo "$expected"
		echo "---------------------"
		echo "But got:"
		echo "---------------------"
		echo "$output"
		echo "---------------------"
		exit 1
	fi
}

function run_test_scip {
	pushd test-projects/$1
	rm -rf target
	rust-analyzer scip . --output target/index.scip
	popd
	cargo run test-projects/$1/target/index.scip > test-projects/$1/target/$1.stdout
	output=$(cat test-projects/$1/target/$1.stdout)
	expected=$(cat test-projects/$1/$1.stdout)
	if [ "$expected" != "$output" ]; then
		echo "Mismatch. Expected:"
		echo "---------------------"
		echo "$expected"
		echo "---------------------"
		echo "But got:"
		echo "---------------------"
		echo "$output"
		echo "---------------------"
		exit 1
	fi
}

run_test_scip test01

run_test_scip test02
