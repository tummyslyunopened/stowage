#!/bin/bash
# Automation script to run cargo tarpaulin in WSL (Debian)
# Usage: bash run_tarpaulin_wsl.sh

set -e

# Ensure Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Ensure tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin
fi

# Move to project directory (assumes script is run from project root)
cd "$(dirname "$0")"

# Run tarpaulin with integration tests and HTML output
cargo tarpaulin --tests --out Html

REPORT=tarpaulin-report.html
if [ -f "$REPORT" ]; then
    echo "Coverage report generated: $REPORT"
else
    echo "Coverage report not found. Check tarpaulin output for errors."
    exit 1
fi
