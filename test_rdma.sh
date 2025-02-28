#!/bin/bash

# Test script for RDMA capabilities on Apple Silicon with Thunderbolt 5

# Set log level (default to info if not provided)
LOG_LEVEL=${1:-info}

echo "===== RDMA Capability Test for Thunderbolt 5 on Apple Silicon ====="
echo "Log level: $LOG_LEVEL"
echo

# Check if prerequisite packages are installed
if ! which pkg-config > /dev/null 2>&1; then
    echo "⚠️ pkg-config not found, which may be needed for RDMA libraries"
    echo "  Installing with brew..."
    brew install pkg-config
fi

# Check for RDMA development libraries (may not be available on macOS)
if [ "$(uname)" = "Darwin" ]; then
    echo "⚠️ Running on macOS, RDMA libraries may not be available"
    echo "  Test will still proceed to check for any available RDMA capabilities"
    echo "  This is an exploratory test for Apple Silicon Thunderbolt 5 RDMA support"
    echo
fi

# Set environment variables
export RUST_LOG=$LOG_LEVEL

# Build and run the test
echo "Building RDMA test utility..."
cargo build --bin test_rdma

if [ $? -eq 0 ]; then
    echo -e "\nRunning RDMA capability test...\n"
    cargo run --bin test_rdma
else
    echo "❌ Failed to build the RDMA test utility"
    exit 1
fi 