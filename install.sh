#!/bin/bash

# Exit on any error
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Node Controller Installation for Apple Silicon Mac${NC}"
echo "Performing system checks..."

# Check for root privileges
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: Please run as root (use sudo)${NC}"
    exit 1
fi

# Identify current user (the user who ran sudo)
REAL_USER=$(who am i | awk '{print $1}')
REAL_USER_HOME=$(eval echo ~$REAL_USER)
echo "Installing for user: $REAL_USER (home: $REAL_USER_HOME)"

# Check for Apple Silicon
if [ "$(uname -m)" != "arm64" ]; then
    echo -e "${RED}Error: This installer is only for Apple Silicon Macs${NC}"
    echo "Detected architecture: $(uname -m)"
    exit 1
fi

# Check for macOS version
if [ "$(sw_vers -productVersion | cut -d. -f1)" -lt 11 ]; then
    echo -e "${RED}Error: macOS 11.0 or later is required${NC}"
    echo "Detected version: $(sw_vers -productVersion)"
    exit 1
fi

# Check for Rust installation
if ! command -v rustc >/dev/null 2>&1; then
    echo -e "${RED}Error: Rust is not installed${NC}"
    echo "Please install Rust using:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check Rust version
RUST_VERSION=$(rustc --version | cut -d' ' -f2)
echo -e "${GREEN}Found Rust version: ${RUST_VERSION}${NC}"

# Check for required Rust target
if ! rustup target list | grep -q "aarch64-apple-darwin (installed)"; then
    echo -e "${YELLOW}Adding aarch64-apple-darwin target...${NC}"
    rustup target add aarch64-apple-darwin
fi

# Check for Xcode Command Line Tools
if ! xcode-select -p >/dev/null 2>&1; then
    echo -e "${RED}Error: Xcode Command Line Tools not found${NC}"
    echo "Please install them using:"
    echo "xcode-select --install"
    exit 1
fi

# Check if cargo can find required dependencies
echo "Checking project dependencies..."
if ! cargo check --quiet; then
    echo -e "${RED}Error: Failed to verify project dependencies${NC}"
    echo "Please ensure all required dependencies are available"
    exit 1
fi

# Check if node-monitor script exists
if [ ! -f "./node-monitor" ]; then
    echo -e "${RED}Error: node-monitor script not found${NC}"
    echo "Please ensure the node-monitor script is in the current directory"
    exit 1
fi

echo -e "${GREEN}All prerequisites met. Starting installation...${NC}"

# Check for existing installation and backup if needed
if [ -d "/Applications/NodeController" ] || [ -f "/Library/LaunchDaemons/com.nodecontroller.daemon.plist" ]; then
    echo -e "${YELLOW}Existing installation detected. Creating backup...${NC}"
    BACKUP_DIR="/Library/NodeController/backup_$(date +%Y%m%d%H%M%S)"
    mkdir -p "${BACKUP_DIR}"
    
    # Backup existing files
    if [ -d "/Applications/NodeController" ]; then
        cp -R /Applications/NodeController "${BACKUP_DIR}/"
    fi
    
    if [ -f "/Library/LaunchDaemons/com.nodecontroller.daemon.plist" ]; then
        cp /Library/LaunchDaemons/com.nodecontroller.daemon.plist "${BACKUP_DIR}/"
        
        # Unload existing service
        echo "Stopping existing service..."
        launchctl unload /Library/LaunchDaemons/com.nodecontroller.daemon.plist || true
    fi
    
    echo -e "${GREEN}Backup created at ${BACKUP_DIR}${NC}"
fi

# Create system directories (require root)
echo "Creating system directories..."
mkdir -p /Applications/NodeController/bin
mkdir -p /Library/NodeController/config
mkdir -p /Library/Logs/NodeController
mkdir -p /usr/local/bin

# Create user-writable directories for updates
echo "Creating user directories for updates..."
USER_UPDATE_DIR="$REAL_USER_HOME/Library/Application Support/NodeController/updates"
mkdir -p "$USER_UPDATE_DIR"
chown -R "$REAL_USER:staff" "$REAL_USER_HOME/Library/Application Support/NodeController"
chmod -R 755 "$REAL_USER_HOME/Library/Application Support/NodeController"

# Build the release binary
echo "Building release binary..."
if ! cargo build --release --target aarch64-apple-darwin; then
    echo -e "${RED}Error: Build failed${NC}"
    exit 1
fi

# Copy binary
echo "Installing binary..."
cp ./target/aarch64-apple-darwin/release/node-controller /Applications/NodeController/bin/
chmod +x /Applications/NodeController/bin/node-controller

# Copy and setup command-line tool
echo "Installing command-line tool..."
cp ./node-monitor /usr/local/bin/
chmod +x /usr/local/bin/node-monitor

# Create default configuration
echo "Creating default configuration..."
cat > /Library/NodeController/config/config.json << EOF
{
  "log_level": "info",
  "update_interval_seconds": 60,
  "enable_performance_monitoring": true,
  "enable_network_monitoring": true,
  "enable_process_monitoring": true,
  "max_log_size_mb": 100,
  "max_log_files": 5,
  "installation_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "installation_version": "0.1.0"
}
EOF

# Get API configuration
echo -e "${GREEN}Monitoring API Configuration${NC}"
read -p "Enter the monitoring API endpoint [https://node-metrics.a14a.org]: " API_ENDPOINT
API_ENDPOINT=${API_ENDPOINT:-https://node-metrics.a14a.org}

read -p "Enter your API key: " API_KEY
if [ -z "$API_KEY" ]; then
    echo -e "${RED}Error: API key is required${NC}"
    echo "Please restart the installation and provide a valid API key"
    exit 1
fi

# Create environment file with user update directory
echo "Creating environment configuration..."
cat > /Library/NodeController/config/.env << EOF
# Monitoring API Configuration
MONITORING_API_URL=${API_ENDPOINT}
MONITORING_API_KEY=${API_KEY}

# Logging Configuration
RUST_LOG=info

# Update Configuration
UPDATE_DIR=$USER_UPDATE_DIR
AUTO_UPDATE=true
EOF

# Copy launch daemon
echo "Installing launch daemon..."
cp ./com.nodecontroller.daemon.plist /Library/LaunchDaemons/
chmod 644 /Library/LaunchDaemons/com.nodecontroller.daemon.plist

# Set proper ownership
echo "Setting permissions..."
chown -R root:wheel /Applications/NodeController
chown -R root:wheel /Library/NodeController
chown -R root:wheel /Library/LaunchDaemons/com.nodecontroller.daemon.plist
chown root:wheel /usr/local/bin/node-monitor

# Create log directory with proper permissions
chmod 755 /Library/Logs/NodeController

# Load the launch daemon
echo "Starting service..."
if ! launchctl load /Library/LaunchDaemons/com.nodecontroller.daemon.plist; then
    echo -e "${RED}Warning: Failed to start service${NC}"
    echo "You can try starting it manually with: sudo node-monitor start"
else
    # Verify service is running
    sleep 2
    if launchctl list | grep -q "${SERVICE_NAME}"; then
        echo -e "${GREEN}Service started successfully${NC}"
    else
        echo -e "${YELLOW}Warning: Service may not have started properly${NC}"
        echo "Check status with: node-monitor status"
    fi
fi

echo -e "${GREEN}Installation complete!${NC}"
echo "The Node Controller service has been installed and started."
echo "Logs can be found in /Library/Logs/NodeController/"
echo "Configuration is at /Library/NodeController/config/config.json"
echo "Updates will be stored in $USER_UPDATE_DIR"
echo ""
echo "You can now use the following commands:"
echo "  node-monitor start    - Start the service"
echo "  node-monitor stop     - Stop the service"
echo "  node-monitor status   - Check service status"
echo "  node-monitor logs     - View logs"
echo "  node-monitor help     - Show all commands" 