#!/bin/bash

# Node Controller Deployment Script
# This script builds and runs the node-controller-rust application

set -e  # Exit on error

# Configuration
LOG_FILE="node-controller.log"
ENV_FILE=".env"
ENV_EXAMPLE=".env.example"
MONITOR_SCRIPT="node-monitor"

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

print_blue() {
    echo -e "\033[0;34m$1\033[0m"
}

# Parse command line arguments
DRY_RUN=false
SYMLINK_BIN=false

for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --symlink)
            SYMLINK_BIN=true
            shift
            ;;
        *)
            # Unknown option
            ;;
    esac
done

# Check system architecture
print_blue "Checking system compatibility..."
architecture=$(uname -m)
os=$(uname -s)

if [ "$os" != "Darwin" ]; then
    print_red "Error: This application is designed for macOS. Current OS: $os"
    exit 1
fi

if [ "$architecture" = "arm64" ]; then
    print_green "✓ Apple Silicon architecture detected."
else
    print_yellow "⚠ Intel architecture detected. Some features may be optimized for Apple Silicon."
    read -p "Do you want to continue? (y/n): " CONTINUE
    if [ "$CONTINUE" != "y" ]; then
        print_yellow "Deployment canceled."
        exit 0
    fi
fi

# Check for Rust installation
print_blue "Checking for Rust installation..."
if ! command -v rustc &> /dev/null || ! command -v cargo &> /dev/null; then
    print_red "Error: Rust is not installed or not in PATH."
    print_yellow "Please install Rust using the following command:"
    print_yellow "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    print_yellow "Then restart your shell and run this script again."
    exit 1
fi

rustc_version=$(rustc --version | awk '{print $2}')
print_green "✓ Rust version $rustc_version detected."

# Check if .env file exists
if [ ! -f "$ENV_FILE" ]; then
    print_yellow "Warning: .env file not found. Creating a new one."
    
    # Prompt for required API configuration
    print_blue "=== Monitoring API Configuration ==="
    read -p "Enter the monitoring API endpoint [https://node-metrics.a14a.org]: " API_ENDPOINT
    API_ENDPOINT=${API_ENDPOINT:-https://node-metrics.a14a.org}
    
    read -p "Enter your API key: " API_KEY
    while [ -z "$API_KEY" ]; do
        print_red "Error: API key is required"
        read -p "Enter your API key: " API_KEY
    done
    
    # Additional configuration with defaults
    print_blue "=== Logging Configuration ==="
    read -p "Log level (error, warn, info, debug, trace) [info]: " LOG_LEVEL
    LOG_LEVEL=${LOG_LEVEL:-info}
    
    print_blue "=== Auto-Update Configuration ==="
    read -p "Enable automatic updates (true/false) [true]: " AUTO_UPDATE
    AUTO_UPDATE=${AUTO_UPDATE:-true}
    
    read -p "Update channel (stable, beta, nightly) [stable]: " UPDATE_CHANNEL
    UPDATE_CHANNEL=${UPDATE_CHANNEL:-stable}
    
    read -p "Update check interval in minutes [60]: " UPDATE_INTERVAL
    UPDATE_INTERVAL=${UPDATE_INTERVAL:-60}
    
    read -p "GitHub repository for updates [a14a-org/node-controller-rust]: " UPDATE_REPO
    UPDATE_REPO=${UPDATE_REPO:-a14a-org/node-controller-rust}
    
    cat > "$ENV_FILE" << EOF
# Node Controller Configuration
MONITORING_API_URL=${API_ENDPOINT}
MONITORING_API_KEY=${API_KEY}

# Logging configuration
RUST_LOG=${LOG_LEVEL}

# Auto-update configuration
AUTO_UPDATE=${AUTO_UPDATE}
UPDATE_CHANNEL=${UPDATE_CHANNEL}
UPDATE_CHECK_INTERVAL=${UPDATE_INTERVAL}
UPDATE_REPOSITORY=${UPDATE_REPO}
EOF
    print_green "✓ .env file created successfully!"
else
    print_green "✓ Found existing .env file."
    read -p "Do you want to update the existing .env file? (y/n): " UPDATE_ENV
    if [ "$UPDATE_ENV" = "y" ]; then
        # Load existing values if any
        if [ -f "$ENV_FILE" ]; then
            source "$ENV_FILE" 2>/dev/null || true
        fi
        
        # Prompt for required API configuration
        print_blue "=== Monitoring API Configuration ==="
        read -p "Enter the monitoring API endpoint [${MONITORING_API_URL:-https://node-metrics.a14a.org}]: " API_ENDPOINT
        API_ENDPOINT=${API_ENDPOINT:-${MONITORING_API_URL:-https://node-metrics.a14a.org}}
        
        read -p "Enter your API key [${MONITORING_API_KEY:-}]: " API_KEY
        if [ -z "$API_KEY" ] && [ -z "$MONITORING_API_KEY" ]; then
            while [ -z "$API_KEY" ]; do
                print_red "Error: API key is required"
                read -p "Enter your API key: " API_KEY
            done
        fi
        API_KEY=${API_KEY:-$MONITORING_API_KEY}
        
        # Additional configuration with defaults
        print_blue "=== Logging Configuration ==="
        read -p "Log level (error, warn, info, debug, trace) [${RUST_LOG:-info}]: " LOG_LEVEL
        LOG_LEVEL=${LOG_LEVEL:-${RUST_LOG:-info}}
        
        print_blue "=== Auto-Update Configuration ==="
        read -p "Enable automatic updates (true/false) [${AUTO_UPDATE:-true}]: " NEW_AUTO_UPDATE
        AUTO_UPDATE=${NEW_AUTO_UPDATE:-${AUTO_UPDATE:-true}}
        
        read -p "Update channel (stable, beta, nightly) [${UPDATE_CHANNEL:-stable}]: " NEW_UPDATE_CHANNEL
        UPDATE_CHANNEL=${NEW_UPDATE_CHANNEL:-${UPDATE_CHANNEL:-stable}}
        
        read -p "Update check interval in minutes [${UPDATE_CHECK_INTERVAL:-60}]: " NEW_UPDATE_INTERVAL
        UPDATE_INTERVAL=${NEW_UPDATE_INTERVAL:-${UPDATE_CHECK_INTERVAL:-60}}
        
        read -p "GitHub repository for updates [${UPDATE_REPOSITORY:-a14a-org/node-controller-rust}]: " NEW_UPDATE_REPO
        UPDATE_REPO=${NEW_UPDATE_REPO:-${UPDATE_REPOSITORY:-a14a-org/node-controller-rust}}
        
        cat > "$ENV_FILE" << EOF
# Node Controller Configuration
MONITORING_API_URL=${API_ENDPOINT}
MONITORING_API_KEY=${API_KEY}

# Logging configuration
RUST_LOG=${LOG_LEVEL}

# Auto-update configuration
AUTO_UPDATE=${AUTO_UPDATE}
UPDATE_CHANNEL=${UPDATE_CHANNEL}
UPDATE_CHECK_INTERVAL=${UPDATE_INTERVAL}
UPDATE_REPOSITORY=${UPDATE_REPO}
EOF
        print_green "✓ .env file updated successfully!"
    fi
fi

if [ "$DRY_RUN" = true ]; then
    print_yellow "Dry run mode. Exiting without building or starting the service."
    exit 0
fi

# Build the application
print_blue "Building node-controller-rust in release mode..."
cargo build --release

# Check if the build was successful
if [ $? -ne 0 ]; then
    print_red "Build failed. Exiting."
    exit 1
fi
print_green "✓ Build completed successfully."

# Make the control script executable
chmod +x "$MONITOR_SCRIPT"
print_green "✓ Control script ($MONITOR_SCRIPT) is ready."

# Symlink the control script to /usr/local/bin if requested
if [ "$SYMLINK_BIN" = true ]; then
    print_blue "Creating symlink to $MONITOR_SCRIPT in /usr/local/bin..."
    
    if [ ! -d "/usr/local/bin" ]; then
        print_yellow "Creating /usr/local/bin directory..."
        sudo mkdir -p /usr/local/bin
    fi
    
    SCRIPT_PATH="$(cd "$(dirname "$0")" && pwd)/$MONITOR_SCRIPT"
    
    if [ -L "/usr/local/bin/$MONITOR_SCRIPT" ] || [ -f "/usr/local/bin/$MONITOR_SCRIPT" ]; then
        print_yellow "Removing existing symlink or file..."
        sudo rm -f "/usr/local/bin/$MONITOR_SCRIPT"
    fi
    
    sudo ln -s "$SCRIPT_PATH" "/usr/local/bin/$MONITOR_SCRIPT"
    print_green "✓ Symlink created. You can now use '$MONITOR_SCRIPT' command from anywhere."
else
    print_yellow "To make $MONITOR_SCRIPT available system-wide, run:"
    print_yellow "    sudo ln -s \"$(cd "$(dirname "$0")" && pwd)/$MONITOR_SCRIPT\" /usr/local/bin/$MONITOR_SCRIPT"
    print_yellow "Or run this script with the --symlink option."
fi

# Use the node-monitor script to start the service
print_blue "Starting node-controller using $MONITOR_SCRIPT..."
./$MONITOR_SCRIPT start

# Get the PID from the monitor script
PID=""
if [ -f ".node-controller.pid" ]; then
    PID=$(cat .node-controller.pid)
fi

# Print instructions
cat << EOF

--------------------------------------------------------------------------------
                       NODE CONTROLLER DEPLOYMENT
--------------------------------------------------------------------------------

The node controller is now running in the background${PID:+ with PID $PID}.
Logs are being written to $LOG_FILE

You can manage the controller with the following commands:

    ./$MONITOR_SCRIPT status   # Check the status
    ./$MONITOR_SCRIPT start    # Start the service
    ./$MONITOR_SCRIPT stop     # Stop the service
    ./$MONITOR_SCRIPT restart  # Restart the service
    ./$MONITOR_SCRIPT logs     # View logs
    ./$MONITOR_SCRIPT update   # Check for and apply updates

To make the control script available system-wide, run:
    $0 --symlink

--------------------------------------------------------------------------------
EOF 