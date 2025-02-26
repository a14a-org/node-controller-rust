# macOS Installation Guide for Node Controller

This guide provides instructions for installing and running the Node Controller as a system service on macOS.

## Prerequisites

- macOS 11.0 or higher (Big Sur or newer)
- Command Line Tools for Xcode: `xcode-select --install`
- Rust toolchain: [Install Rust](https://www.rust-lang.org/tools/install)
- Apple Silicon Mac (M1/M2/M3)

## Installation Steps

### 1. Clone the Repository

```bash
git clone https://github.com/a14a-org/node-controller-rust.git
cd node-controller-rust
```

### 2. Automated Installation

The simplest way to install is using the provided installation script:

```bash
sudo ./install.sh
```

The script will:
- Check system requirements
- Build the application
- Install it as a system service
- Prompt for your monitoring API credentials
- Configure automatic startup
- Create necessary directories and set permissions

### 3. Manual Installation (Alternative)

If you prefer to install manually, follow these steps:

#### 3.1 Build the Application

```bash
cargo build --release
```

#### 3.2 Create Installation Directories

```bash
# Create directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /usr/local/etc/node-controller
sudo mkdir -p /Library/Logs/NodeController
```

#### 3.3 Copy Files

```bash
# Copy the executable
sudo cp target/release/node-controller-rust /usr/local/bin/

# Copy the launchd plist file
sudo cp org.a14a.node-controller.plist /Library/LaunchDaemons/

# Create configuration directory and .env file
sudo cp .env.example /usr/local/etc/node-controller/.env
```

#### 3.4 Configure the Application

Edit the environment file with your API credentials:

```bash
sudo nano /usr/local/etc/node-controller/.env
```

Add the following content (replace with your actual API key):

```
MONITORING_API_URL=https://node-metrics.a14a.org
MONITORING_API_KEY=your-api-key
RUST_LOG=info
```

#### 3.5 Set Proper Permissions

```bash
# Set ownership and permissions
sudo chown root:wheel /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo chmod 644 /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo chown root:wheel /usr/local/bin/node-controller-rust
sudo chmod 755 /usr/local/bin/node-controller-rust
```

#### 3.6 Load and Start the Service

```bash
# Load the service
sudo launchctl load /Library/LaunchDaemons/org.a14a.node-controller.plist

# Start the service
sudo launchctl start org.a14a.node-controller
```

## Auto-Update Feature

The Node Controller includes an automatic update system that:

- Periodically checks for new releases on GitHub
- Downloads updates securely when available
- Verifies the integrity of downloaded updates
- Creates backups before applying updates
- Automatically rolls back in case of failed updates
- Can be configured to notify only (without auto-updating)

The update system uses GitHub releases tagged with specific release channels (stable, beta), allowing you to control which updates are applied to your system.

## Verifying Installation

Check if the service is running:

```bash
sudo launchctl list | grep org.a14a.node-controller
```

Check the log file:

```bash
tail -f /Library/Logs/NodeController/node-controller.log
```

## Managing the Service

### Using the CLI Tool

The node-monitor tool provides a convenient way to manage the service:

```bash
node-monitor status  # Check service status
node-monitor start   # Start the service
node-monitor stop    # Stop the service
node-monitor restart # Restart the service
node-monitor logs    # View logs
```

### Manual Service Management

```bash
# Stop the service
sudo launchctl stop org.a14a.node-controller

# Unload the service
sudo launchctl unload /Library/LaunchDaemons/org.a14a.node-controller.plist

# Reload and restart the service
sudo launchctl unload /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo launchctl load /Library/LaunchDaemons/org.a14a.node-controller.plist
```

## Uninstallation

To completely remove the Node Controller:

```bash
# Use the uninstall script (recommended)
sudo ./uninstall.sh

# Or manually:
sudo launchctl unload /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo rm /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo rm -rf /Applications/NodeController
sudo rm -rf /Library/NodeController
sudo rm /usr/local/bin/node-monitor
```

## Troubleshooting

### Service Not Starting

Check the system log for launchd-related errors:

```bash
sudo log show --predicate 'subsystem == "com.apple.launchd"' --last 1h
```

### Permission Issues

Ensure all files have the correct ownership and permissions as specified in the installation steps.

### API Connection Issues

Verify that the API URL and key in the `.env` file are correct and that the Mac has internet connectivity to reach the API endpoint.

## Author

Developed by D.A.F. Mulder (dafmulder@gmail.com) 