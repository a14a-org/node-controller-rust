# Node Controller Rust

A lightweight system monitoring agent written in Rust for Mac clusters. This application collects system metrics including CPU, memory, network, and storage information and sends them to a central monitoring API.

## Features

- Collects detailed system information:
  - CPU usage and load
  - Memory usage
  - Network statistics
  - Storage information
  - System details (OS, kernel, architecture)
- Low resource footprint
- Configurable update intervals
- Secure API communication
- Automatic updates from GitHub releases

## Requirements

- Rust 1.70 or higher
- macOS 11.0 or higher (Apple Silicon or Intel)
- Internet connection for API communication

## Installation Options

The Node Controller can be installed and run in two different ways:

### Option 1: System Installation (Recommended for Production)

The system installation sets up Node Controller as a system service that starts automatically on boot and runs with the necessary privileges.

1. Clone the repository:
   ```
   git clone https://github.com/a14a-org/node-controller-rust.git
   cd node-controller-rust
   ```

2. Run the installation script with sudo:
   ```
   sudo ./install.sh
   ```

The `install.sh` script:
- Installs the application as a system service
- Configures it to run at system startup
- Creates the necessary directories with appropriate permissions
- Sets up automatic updates that don't require elevated privileges
- Requires sudo only during installation (one-time)

### Option 2: Development/Testing Deployment

For development or testing purposes, you can use the deployment script which runs the application in the current directory.

1. Clone the repository:
   ```
   git clone https://github.com/a14a-org/node-controller-rust.git
   cd node-controller-rust
   ```

2. Configure the application by creating a `.env` file:
   ```
   MONITORING_API_URL=https://monitoring.a14a.org/api
   MONITORING_API_KEY=your-api-key
   RUST_LOG=info
   AUTO_UPDATE=true
   UPDATE_CHANNEL=stable
   UPDATE_CHECK_INTERVAL=60
   ```

3. Run the deployment script:
   ```
   ./deploy.sh
   ```

The `deploy.sh` script:
- Builds the application in the current directory
- Runs it without installing as a system service
- Is ideal for testing and development
- Doesn't require sudo privileges

## Manual Deployment

If you prefer to deploy manually:

1. Build the application:
   ```
   cargo build --release
   ```

2. Run the application:
   ```
   ./target/release/node-controller-rust > node-controller.log 2>&1 &
   ```

## Configuration

The application uses environment variables for configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| MONITORING_API_URL | URL of the monitoring API | http://localhost:3000 |
| MONITORING_API_KEY | API key for authentication | dev-api-key |
| RUST_LOG | Logging level (error, warn, info, debug, trace) | info |
| AUTO_UPDATE | Enable automatic updates from GitHub releases | true |
| UPDATE_CHANNEL | Update channel to use (stable, beta, nightly) | stable |
| UPDATE_CHECK_INTERVAL | How often to check for updates (minutes) | 60 |
| UPDATE_REPOSITORY | GitHub repository for updates | a14a-org/node-controller-rust |
| UPDATE_DIR | Directory for updates and backups | ~/Library/Application Support/NodeController/updates |

## Auto-Update System

The node controller includes an automatic update system that can check for and apply updates from GitHub releases:

- Updates are downloaded securely from GitHub releases
- The current version is backed up before updating
- Health checks ensure the update was successful
- Automatic rollback if an update fails
- Configurable update channels (stable, beta, nightly)
- Updates are stored in the user's Application Support directory and don't require elevated privileges

To create a new release that will be detected by clients:

1. Tag your release with the format `{channel}-{version}`, e.g., `stable-0.2.0`
2. Upload the binary as an asset to the GitHub release
3. Clients will automatically detect and apply the update based on their configuration

## Deployment on Mac Cluster

For deploying across a Mac cluster:

1. Ensure Rust is installed on each node
2. Copy the application to each node
3. Use `sudo ./install.sh` to install the service on each node
4. Provide API credentials during installation

For automated deployment, consider using a configuration management tool like Ansible.

## Troubleshooting

- **Application not starting**: Check the log file for errors
- **API connection issues**: Verify the API URL and key in the `.env` file
- **High resource usage**: Check for abnormal system activity
- **Update failures**: Check logs for update errors and ensure the application has proper permissions

## Development

To build and run in development mode:

```
cargo build
cargo run
```

## License

MIT

## Author

Developed by D.A.F. Mulder (dafmulder@gmail.com) 

## File Transfer and RDMA Testing

The Node Controller includes utilities for high-performance file transfers between nodes, with two implementation options:
- Optimized TCP-based file transfer (available on all platforms)
- RDMA-based transfer (requires compatible hardware)

### Testing File Transfer Capabilities

1. First, check if your system supports RDMA:
   ```
   ./test_rdma_compat.sh
   ```
   This will analyze your system and determine the best file transfer method available.

2. To test the file transfer functionality between nodes:
   
   On the first node:
   ```
   # Build and start the file transfer utility
   cargo run --bin test_file_transfer
   ```

   On the second node:
   ```
   # Build and start the file transfer utility
   cargo run --bin test_file_transfer
   ```

3. Once both nodes are running the utility:
   - Use the `list` command to discover other nodes
   - Use `send <node_id> <file_path>` to transfer a file to another node
   - Files are received in the system's temp directory under `node_controller_files`

### File Transfer Features

- **Auto-Discovery**: Nodes automatically discover each other using mDNS
- **Multi-Stream Transfer**: Uses multiple concurrent streams for maximum throughput
- **Progress Reporting**: Real-time progress updates during transfers
- **Resilient Transfers**: Built-in error handling and recovery
- **Optimized for Performance**: Uses buffer pools and other optimizations 