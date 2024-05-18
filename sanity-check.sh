#!/bin/sh

set -ex

# Simple script to just run some one-commit sanity checks
cargo +nightly clippy --all-targets
cargo +nightly clippy --all-features --all-targets
cargo +nightly fmt --check

cargo test
cargo test --all-features
cargo +1.56.1 test
cargo +1.56.1 test --all-features

export RUSTDOCFLAGS="--cfg docsrs -D warnings -D rustdoc::broken-intra-doc-links"
cargo +nightly doc --all-features
export RUSTDOCFLAGS="-D warnings"
cargo +nightly doc --all-features

