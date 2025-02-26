# macOS Installation Guide for Node Controller

This guide provides instructions for installing and running the Node Controller as a system service on macOS.

## Prerequisites

- macOS 11.0 or higher (Big Sur or newer)
- Command Line Tools for Xcode: `xcode-select --install`
- Rust toolchain: [Install Rust](https://www.rust-lang.org/tools/install)

## Installation Steps

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/node-controller-rust.git
cd node-controller-rust
```

### 2. Build the Application

```bash
cargo build --release
```

### 3. Create Installation Directories

```bash
# Create directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /usr/local/etc/node-controller
sudo mkdir -p /Library/Logs
```

### 4. Copy Files

```bash
# Copy the executable
sudo cp target/release/node-controller-rust /usr/local/bin/

# Copy the launchd plist file
sudo cp org.a14a.node-controller.plist /Library/LaunchDaemons/

# Create configuration directory and .env file
sudo cp .env /usr/local/etc/node-controller/
```

### 5. Configure the Application

Edit the environment file with your API credentials:

```bash
sudo nano /usr/local/etc/node-controller/.env
```

Add the following content (replace with your actual API key):

```
MONITORING_API_URL=https://monitoring.a14a.org/api
MONITORING_API_KEY=your-api-key
RUST_LOG=info
```

### 6. Set Proper Permissions

```bash
# Set ownership and permissions
sudo chown root:wheel /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo chmod 644 /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo chown root:wheel /usr/local/bin/node-controller-rust
sudo chmod 755 /usr/local/bin/node-controller-rust
```

### 7. Load and Start the Service

```bash
# Load the service
sudo launchctl load /Library/LaunchDaemons/org.a14a.node-controller.plist

# Start the service
sudo launchctl start org.a14a.node-controller
```

## Verifying Installation

Check if the service is running:

```bash
sudo launchctl list | grep org.a14a.node-controller
```

Check the log file:

```bash
tail -f /Library/Logs/node-controller.log
```

## Managing the Service

### Stopping the Service

```bash
sudo launchctl stop org.a14a.node-controller
```

### Unloading the Service

```bash
sudo launchctl unload /Library/LaunchDaemons/org.a14a.node-controller.plist
```

### Restarting the Service

```bash
sudo launchctl unload /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo launchctl load /Library/LaunchDaemons/org.a14a.node-controller.plist
```

## Uninstallation

To completely remove the Node Controller:

```bash
# Unload the service
sudo launchctl unload /Library/LaunchDaemons/org.a14a.node-controller.plist

# Remove files
sudo rm /Library/LaunchDaemons/org.a14a.node-controller.plist
sudo rm /usr/local/bin/node-controller-rust
sudo rm -rf /usr/local/etc/node-controller
```

## Troubleshooting

### Service Not Starting

Check the system log for launchd-related errors:

```bash
sudo log show --predicate 'subsystem == "com.apple.launchd"' --last 1h
```

### Permission Issues

Ensure all files have the correct ownership and permissions as specified in step 6.

### API Connection Issues

Verify that the API URL and key in the `.env` file are correct and that the Mac has internet connectivity to reach the API endpoint. 