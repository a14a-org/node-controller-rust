#!/bin/bash

# Node Controller Deployment Script
# This script builds and runs the node-controller-rust application

set -e  # Exit on error

# Configuration
LOG_FILE="node-controller.log"
ENV_FILE=".env"

# Print colored output
print_green() {
    echo -e "\033[0;32m$1\033[0m"
}

print_yellow() {
    echo -e "\033[0;33m$1\033[0m"
}

print_red() {
    echo -e "\033[0;31m$1\033[0m"
}

# Check if .env file exists
if [ ! -f "$ENV_FILE" ]; then
    print_yellow "Warning: .env file not found. Creating a new one."
    
    # Prompt for API configuration
    print_green "Monitoring API Configuration"
    read -p "Enter the monitoring API endpoint [https://node-metrics.a14a.org]: " API_ENDPOINT
    API_ENDPOINT=${API_ENDPOINT:-https://node-metrics.a14a.org}
    
    read -p "Enter your API key: " API_KEY
    if [ -z "$API_KEY" ]; then
        print_red "Error: API key is required"
        print_yellow "Please restart the deployment and provide a valid API key"
        exit 1
    fi
    
    cat > "$ENV_FILE" << EOF
# Node Controller Configuration
MONITORING_API_URL=${API_ENDPOINT}
MONITORING_API_KEY=${API_KEY}

# Logging configuration
RUST_LOG=info
EOF
    print_green ".env file created successfully!"
else
    print_green "Found existing .env file."
fi

# Build the application
print_green "Building node-controller-rust in release mode..."
cargo build --release

# Check if the build was successful
if [ $? -ne 0 ]; then
    print_red "Build failed. Exiting."
    exit 1
fi

# Check if the application is already running
PID=$(pgrep -f "node-controller-rust" || echo "")
if [ ! -z "$PID" ]; then
    print_yellow "Node controller is already running with PID $PID."
    read -p "Do you want to stop it and start a new instance? (y/n): " RESTART
    if [ "$RESTART" = "y" ]; then
        print_yellow "Stopping existing process..."
        kill $PID
        sleep 2
    else
        print_yellow "Exiting without starting a new instance."
        exit 0
    fi
fi

# Start the application
print_green "Starting node-controller-rust..."
./target/release/node-controller-rust > "$LOG_FILE" 2>&1 &
NEW_PID=$!

print_green "Node controller started with PID $NEW_PID"
print_green "Logs are being written to $LOG_FILE"
print_yellow "You can view logs with: tail -f $LOG_FILE"

# Print instructions
cat << EOF

--------------------------------------------------------------------------------
                       NODE CONTROLLER DEPLOYMENT
--------------------------------------------------------------------------------

The node controller is now running in the background with PID $NEW_PID.

To stop the controller:
    kill $NEW_PID

To view logs:
    tail -f $LOG_FILE

To configure the controller, edit the .env file with your settings.

--------------------------------------------------------------------------------
EOF 