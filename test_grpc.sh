#!/bin/bash

# Get node name from command line or use default
NODE_NAME=${1:-"$(hostname)"}
LOG_LEVEL=${2:-"info"}

echo "Starting gRPC Communication Test with name: $NODE_NAME"
echo "Log level: $LOG_LEVEL"

# Set environment variables
export RUST_LOG=$LOG_LEVEL

# Build and run the test binary
cargo build --bin test_grpc
if [ $? -eq 0 ]; then
    # Run the test binary
    ./target/debug/test_grpc $NODE_NAME
else
    echo "Failed to build test_grpc binary."
    exit 1
fi 