#!/bin/bash

# Test script for optimized TCP file transfers

# Set log level (default to info if not provided)
LOG_LEVEL=${1:-info}

echo "===== High-Performance File Transfer Test ====="
echo "Log level: $LOG_LEVEL"
echo

# Set environment variables
export RUST_LOG=$LOG_LEVEL

# Build and run the test
echo "Building file transfer test utility..."
cargo build --bin test_file_transfer

if [ $? -eq 0 ]; then
    echo -e "\nStarting file transfer test utility...\n"
    cargo run --bin test_file_transfer
else
    echo "❌ Failed to build the file transfer test utility"
    exit 1
fi 