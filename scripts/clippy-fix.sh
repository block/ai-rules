#!/bin/bash
set -e

echo "Running Clippy auto-fix..."
cargo clippy --all-targets --all-features --fix