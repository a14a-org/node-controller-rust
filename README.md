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

## File Transfer and Node Discovery

The Node Controller includes a robust system for node discovery and high-performance file transfers between nodes in a cluster:

### Node Discovery

Nodes automatically discover each other using mDNS (multicast DNS) service discovery:

- **Zero Configuration**: No manual setup required - nodes find each other automatically
- **Interface Optimization**: Prioritizes fastest network interfaces (Thunderbolt > Ethernet > WiFi)
- **Real-Time Updates**: Continuously discovers new nodes and removes stale ones
- **Unique Identification**: Each node has a unique ID and friendly name for easy reference

### High-Performance File Transfer

Two implementation options are available for file transfers:

1. **Optimized TCP-based Transfer** (Available on all platforms)
   - Uses multiple parallel TCP streams for maximum throughput (default: 4 streams)
   - Implements buffer pooling and other optimizations for high performance
   - File integrity verification using SHA256 hash
   - Progress reporting and throughput statistics

2. **RDMA-based Transfer** (Requires compatible hardware)
   - Leverages Remote Direct Memory Access for near line-speed transfers
   - Bypasses CPU involvement in data movement
   - Automatically falls back to TCP if RDMA is unavailable

### Testing File Discovery and Transfer

To test the node discovery and file transfer capabilities:

1. Build and run the test utility on two or more nodes:
   ```
   cargo run --bin test_file_transfer
   ```

2. Use the interactive commands to discover and transfer files:
   ```
   # List all discovered nodes on the network
   > list
   
   # Sample output:
   Discovered nodes:
     1. macmini-lab3 (797c0136)
        Address: 192.168.1.103:54321
     2. macpro-render (58af92c1)
        Address: 192.168.1.105:54321
   
   # Send a file to another node (using node ID)
   > send 797c0136 /tmp/test_10mb.bin
   
   # Send a file to another node (using node name)
   > send macpro-render /path/to/large_dataset.zip
   ```

3. Monitor transfer progress:
   ```
   ⬆️ Transfer started: large_dataset.zip (256.35 MB)
   📊 Transfer progress: 10.0% (25.63/256.35 MB)
   ...
   📊 Transfer progress: 100.0% (256.35/256.35 MB)
   ✅ Transfer completed: 256.35 MB in 5.67s (45.21 MB/s)
   ```

4. Received files are stored in the system's temp directory:
   ```
   # Default location on macOS
   ~/Library/Application Support/NodeController/received_files/
   
   # Default location on Linux
   /tmp/node_controller_files/
   ```

### Technical Features

The file transfer system includes several technical optimizations:

- **Multi-Stream Transfers**: Divides files into ranges sent over separate TCP streams
- **Hash Verification**: Calculates SHA256 hash to verify file integrity
- **Partial Transfer Support**: Can resume interrupted transfers
- **Buffer Pooling**: Reuses memory buffers to reduce allocation overhead
- **Concurrent Streams**: Configurable number of parallel connections
- **Progress Monitoring**: Real-time tracking of transfer progress

For even higher performance on compatible hardware, the RDMA implementation can be enabled with the `rdma` feature flag 