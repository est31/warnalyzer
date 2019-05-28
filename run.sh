#!/usr/bin/env bash

pushd test-projects/test01
rm -rf target
RUSTFLAGS="-Z save-analysis" cargo +nightly check
popd
cargo run test-projects/test01/target/debug/deps/save-analysis/binary_thing-*.json
