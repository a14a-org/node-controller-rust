#!/bin/bash

# Exit on any error
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Uninstalling Node Controller...${NC}"

# Check for root privileges
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: Please run as root (use sudo)${NC}"
    exit 1
fi

# Unload the launch daemon
if [ -f "/Library/LaunchDaemons/com.nodecontroller.daemon.plist" ]; then
    echo "Stopping service..."
    launchctl unload /Library/LaunchDaemons/com.nodecontroller.daemon.plist || true
fi

# Backup configuration if requested
if [ -d "/Library/NodeController/config" ]; then
    read -p "Do you want to backup the configuration before removing? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        BACKUP_DIR="$HOME/NodeController_backup_$(date +%Y%m%d%H%M%S)"
        echo -e "${YELLOW}Backing up configuration to ${BACKUP_DIR}${NC}"
        mkdir -p "${BACKUP_DIR}"
        cp -R /Library/NodeController/config "${BACKUP_DIR}/"
        echo -e "${GREEN}Configuration backed up to ${BACKUP_DIR}/config${NC}"
    fi
fi

# Remove files
echo "Removing files..."
rm -rf /Applications/NodeController
rm -f /Library/LaunchDaemons/com.nodecontroller.daemon.plist
rm -rf /Library/NodeController
rm -f /usr/local/bin/node-monitor

# Optionally remove logs (ask user)
if [ -d "/Library/Logs/NodeController" ]; then
    read -p "Do you want to remove log files? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf /Library/Logs/NodeController
        echo "Log files removed."
    else
        echo "Log files preserved at /Library/Logs/NodeController"
    fi
fi

echo -e "${GREEN}Uninstallation complete!${NC}" 