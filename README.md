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

## Quick Start

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

3. Build and run using the deployment script:
   ```
   ./deploy.sh
   ```

The script will build the application in release mode and start it in the background with proper logging.

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
| AUTO_UPDATE | Enable automatic updates from GitHub releases | false |
| UPDATE_CHANNEL | Update channel to use (stable, beta, nightly) | stable |
| UPDATE_CHECK_INTERVAL | How often to check for updates (minutes) | 60 |
| UPDATE_REPOSITORY | GitHub repository for updates | a14a-org/node-controller-rust |

## Auto-Update System

The node controller includes an automatic update system that can check for and apply updates from GitHub releases:

- Updates are downloaded securely from GitHub releases
- The current version is backed up before updating
- Health checks ensure the update was successful
- Automatic rollback if an update fails
- Configurable update channels (stable, beta, nightly)

To create a new release that will be detected by clients:

1. Tag your release with the format `{channel}-{version}`, e.g., `stable-0.2.0`
2. Upload the binary as an asset to the GitHub release
3. Clients will automatically detect and apply the update based on their configuration

## Deployment on Mac Cluster

For deploying across a Mac cluster:

1. Ensure Rust is installed on each node
2. Copy the application to each node
3. Configure the `.env` file with appropriate API credentials
4. Run the deployment script on each node

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