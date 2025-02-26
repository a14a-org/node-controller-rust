#!/bin/bash

# Get node name from command line or use default
NODE_NAME=${1:-"$(hostname)"}
LOG_LEVEL=${2:-"info"}

echo "Starting gRPC Communication Test with name: $NODE_NAME"
echo "Log level: $LOG_LEVEL"

# Check if protoc is installed
if ! command -v protoc &> /dev/null; then
    echo "Protocol Buffers compiler (protoc) not found. Attempting to install..."
    
    # Check the OS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS - use Homebrew
        if ! command -v brew &> /dev/null; then
            echo "Error: Homebrew not found. Please install Homebrew or protoc manually."
            echo "To install Homebrew: /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            echo "Or install protoc manually from: https://github.com/protocolbuffers/protobuf/releases"
            exit 1
        fi
        
        echo "Installing protoc via Homebrew..."
        brew install protobuf
        if [ $? -ne 0 ]; then
            echo "Failed to install protoc. Please install it manually."
            exit 1
        fi
        echo "protoc installed successfully."
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux - try apt-get for Debian-based systems
        if command -v apt-get &> /dev/null; then
            echo "Installing protoc via apt-get..."
            sudo apt-get update && sudo apt-get install -y protobuf-compiler
            if [ $? -ne 0 ]; then
                echo "Failed to install protoc. Please install it manually."
                exit 1
            fi
        else
            echo "Error: Could not determine how to install protoc on your Linux distribution."
            echo "Please install protoc manually for your distribution."
            echo "For Debian/Ubuntu: sudo apt-get install protobuf-compiler"
            echo "For Fedora: sudo dnf install protobuf-compiler"
            echo "Or download from: https://github.com/protocolbuffers/protobuf/releases"
            exit 1
        fi
    else
        echo "Error: Unsupported operating system. Please install protoc manually."
        echo "Download from: https://github.com/protocolbuffers/protobuf/releases"
        exit 1
    fi
fi

# Set environment variables
export RUST_LOG=$LOG_LEVEL

# Build and run the test binary
echo "Building the test binary..."
cargo build --bin test_grpc
if [ $? -eq 0 ]; then
    # Run the test binary
    echo "Starting the gRPC test application..."
    ./target/debug/test_grpc $NODE_NAME
else
    echo "Failed to build test_grpc binary."
    exit 1
fi 