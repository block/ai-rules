#!/bin/bash
set -e

echo "Running Clippy checks..."
cargo clippy --all-targets --all-features -- -D warnings