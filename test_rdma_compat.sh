#!/bin/bash

# Test script for RDMA capabilities on Apple Silicon with Thunderbolt 5
# This is a more compatible version that doesn't require RDMA libraries to be installed

# Set log level (default to info if not provided)
LOG_LEVEL=${1:-info}

echo "===== RDMA Capability Test for Thunderbolt 5 on Apple Silicon ====="
echo "Log level: $LOG_LEVEL"
echo
echo "This is a compatibility version that works even without RDMA libraries"
echo

# Check if prerequisite packages are installed
if ! which pkg-config > /dev/null 2>&1; then
    echo "⚠️ pkg-config not found, which may be needed for some system checks"
    echo "  Installing with brew..."
    brew install pkg-config
fi

# Set environment variables
export RUST_LOG=$LOG_LEVEL

# Build and run the test
echo "Building RDMA compatibility test utility..."
cargo build --bin test_rdma_compat

if [ $? -eq 0 ]; then
    echo -e "\nRunning RDMA capability test...\n"
    cargo run --bin test_rdma_compat
else
    echo "❌ Failed to build the RDMA compatibility test utility"
    exit 1
fi 