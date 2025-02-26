# macOS Installation Guide for Node Controller

This guide provides instructions for installing and running the Node Controller on macOS.

## Prerequisites

- macOS 11.0 or higher (Big Sur or newer)
- Command Line Tools for Xcode: `xcode-select --install`
- Rust toolchain: [Install Rust](https://www.rust-lang.org/tools/install)
- Apple Silicon Mac recommended (Intel Macs are supported but some features are optimized for Apple Silicon)

## Installation Steps

### 1. Clone the Repository

```bash
git clone https://github.com/a14a-org/node-controller-rust.git
cd node-controller-rust
```

### 2. Automated Installation

The simplest way to install is using the provided deployment script:

```bash
./deploy.sh
```

The script will:
- Check system compatibility (macOS and architecture)
- Verify Rust installation
- Create or update the `.env` configuration file
- Build the application
- Install the `node-monitor` control script
- Start the Node Controller application
- Provide instructions for managing the service

To make the control script available system-wide:

```bash
./deploy.sh --symlink
```

### 3. Using the Node Monitor Control Script

The `node-monitor` script provides a convenient way to manage the Node Controller:

```bash
./node-monitor status   # Check if the service is running
./node-monitor start    # Start the service
./node-monitor stop     # Stop the service
./node-monitor restart  # Restart the service
./node-monitor logs     # View the last 50 lines of logs
./node-monitor logs 100 # View the last 100 lines of logs
./node-monitor logs -f  # Follow logs in real-time
./node-monitor update   # Check for and apply updates
```

If you installed with the `--symlink` option, you can use the script from anywhere:

```bash
node-monitor status   # Check status from any directory
```

## Configuration

The Node Controller is configured using a `.env` file in the project directory. This file is created or updated during the deployment process with your input.

Key configuration options include:

### Monitoring API Configuration
```
MONITORING_API_URL=https://node-metrics.a14a.org
MONITORING_API_KEY=your-api-key
```

### Logging Configuration
```
RUST_LOG=info  # Options: error, warn, info, debug, trace
```

### Auto-Update Configuration
```
AUTO_UPDATE=true                            # Enable/disable automatic updates
UPDATE_CHANNEL=stable                       # Options: stable, beta, nightly
UPDATE_CHECK_INTERVAL=60                    # Interval in minutes
UPDATE_REPOSITORY=a14a-org/node-controller-rust  # GitHub repository
```

## Auto-Update Feature

The Node Controller includes an automatic update system that:

- Periodically checks for new releases on GitHub
- Downloads updates securely when available
- Verifies the integrity of downloaded updates
- Creates backups before applying updates
- Automatically rolls back in case of failed updates
- Can be configured to notify only (without auto-updating)

The update system uses GitHub releases tagged with specific release channels (stable, beta, nightly), allowing you to control which updates are applied to your system.

## Verifying Installation

Check if the service is running:

```bash
./node-monitor status
```

Check the log file:

```bash
./node-monitor logs -f
```

## Troubleshooting

### Service Not Starting

Check the logs for any error messages:

```bash
./node-monitor logs
```

Common issues include:
- Missing or incorrect API credentials
- Insufficient permissions
- Missing dependencies

### Permission Issues

Ensure that the user running the application has the necessary permissions to access system resources.

### API Connection Issues

Verify that the API URL and key in the `.env` file are correct and that the Mac has internet connectivity to reach the API endpoint.

## Updating the Application

The Node Controller can check for and apply updates automatically:

```bash
./node-monitor update
```

This will check for updates and, if the `AUTO_UPDATE` setting is enabled in your `.env` file, apply them automatically.

## Uninstallation

To completely remove the Node Controller:

1. Stop the service:
   ```bash
   ./node-monitor stop
   ```

2. If you created a system-wide symlink, remove it:
   ```bash
   sudo rm /usr/local/bin/node-monitor
   ```

3. Remove the project directory:
   ```bash
   cd ..
   rm -rf node-controller-rust
   ```

## Advanced Usage

### Dry Run Mode

To test the deployment script without building or starting the service:

```bash
./deploy.sh --dry-run
```

### Custom Log Location

You can modify the log file location in the `node-monitor` script by editing the `LOG_FILE` variable.

## Author

Developed by D.A.F. Mulder (dafmulder@gmail.com) 