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