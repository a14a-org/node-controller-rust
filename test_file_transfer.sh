#!/bin/bash

# Test script for file transfer functionality between nodes
# This script builds and runs the file transfer test utility

# Set log level (default to info if not provided)
LOG_LEVEL=${1:-info}

echo "===== Node Controller File Transfer Test Utility ====="
echo "Log level: $LOG_LEVEL"
echo
echo "This utility allows testing the high-performance file transfer system"
echo "between nodes in the cluster."
echo

# Check if prerequisite packages are installed
if ! which pkg-config > /dev/null 2>&1; then
    echo "⚠️ pkg-config not found, which may be needed for some system checks"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "  Installing with brew..."
        brew install pkg-config
    elif [[ -f /etc/debian_version ]]; then
        echo "  Installing with apt..."
        sudo apt-get install -y pkg-config
    elif [[ -f /etc/redhat-release ]]; then
        echo "  Installing with dnf..."
        sudo dnf install -y pkgconfig
    fi
fi

# Set environment variables
export RUST_LOG=$LOG_LEVEL

# Build and run the test
echo "Building file transfer test utility..."
cargo build --bin test_file_transfer

if [ $? -eq 0 ]; then
    echo -e "\nRunning file transfer test utility...\n"
    # Use the hostname as the node name if none is provided
    NODE_NAME=${2:-$(hostname)}
    cargo run --bin test_file_transfer $NODE_NAME
else
    echo "❌ Failed to build the file transfer test utility"
    exit 1
fi 