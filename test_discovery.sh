#!/bin/bash

# Default values
NODE_NAME=${1:-"test-node"}
LOG_LEVEL=${2:-"info"}

echo "Starting Node Discovery Test with name: $NODE_NAME"
echo "Log level: $LOG_LEVEL"

# Compile the test binary if needed
cargo build --bin test_discovery

# Run the discovery test
RUST_LOG=$LOG_LEVEL ./target/debug/test_discovery "$NODE_NAME" 