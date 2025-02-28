# Node Discovery and Communication

This module implements network discovery and communication capabilities for the Node Controller application, enabling nodes to discover each other automatically and establish the fastest possible connections.

## Features

### Phase 1: Node Discovery (Implemented)
- **Zero-configuration service discovery** using mDNS
- **Automatic interface detection** with preference for Thunderbolt and Ethernet
- **Real-time node monitoring** with automatic cleanup of stale nodes

### Planned Future Phases
- **Phase 2**: gRPC Communication Framework
- **Phase 3**: Interface Optimization and Performance Tuning
- **Phase 4**: Resilience and Recovery Mechanisms

## Configuration

The discovery service can be configured using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `NODE_NAME` | Custom name for this node | System hostname |
| `DISCOVERY_PORT` | Port to use for service discovery | 54321 |

## Usage

### Finding and Using Node Discovery

The node discovery functionality is automatically started in the main application entry point. The service will:

1. Detect the available network interfaces
2. Select the best interface for node communication (prioritizing Thunderbolt > Ethernet > WiFi)
3. Advertise this node's presence on the network using mDNS
4. Continuously discover other nodes on the network

### Accessing Discovered Nodes

```rust
// Example code to access discovered nodes
use networking::NodeDiscovery;

// Create discovery service
let discovery = NodeDiscovery::new("my-node", None)?;

// Start the service 
discovery.start().await?;

// Get list of discovered nodes
let nodes = discovery.get_discovered_nodes();
for node in nodes {
    println!("Found node: {} at {}:{}", node.name, node.ip, node.port);
}
```

## Networking Architecture

The networking module is organized as follows:

- `mod.rs`: Module definition and exports
- `interface.rs`: Network interface detection and classification
- `discovery.rs`: mDNS-based node discovery implementation

## How Interface Detection Works

The system prioritizes interfaces in the following order:
1. **Thunderbolt Bridge** (highest priority) - For fastest node-to-node communication
2. **Ethernet** - For reliable, wired communication
3. **WiFi** - Used when wired connections are unavailable
4. **Loopback** - Used for local testing only

Thunderbolt detection looks for interface names containing "thunderbolt", "tb", or "bridge", as well as certain enumeration patterns. 

## High-Performance File Transfer

The Node Controller includes a high-performance file transfer system implemented in two variants:

1. **Optimized TCP Transfer** (Available on all platforms)
   - Uses multiple parallel TCP streams for maximum throughput
   - Includes configurable buffer sizes and buffer pooling
   - Progress monitoring and reporting
   - Available through the `FileTransferManager` API

2. **RDMA-Based Transfer** (Requires compatible hardware)
   - Leverages Remote Direct Memory Access for near line-speed transfers
   - Bypasses CPU involvement in data movement
   - Automatically falls back to TCP if RDMA is unavailable
   - Best performance on systems with Thunderbolt or InfiniBand connections

### Using the File Transfer API

```rust
// Example code for sending a file
use networking::{FileTransferConfig, FileTransferManager};

// Create and configure the file transfer manager
let config = FileTransferConfig {
    chunk_size: 1024 * 1024, // 1MB chunks
    port: 7879,              // Default port
    receive_dir: std::env::temp_dir().join("received_files"),
    progress_callback: Some(Arc::new(|status| {
        // Handle progress updates
    })),
    concurrent_streams: 4,   // Use 4 parallel streams
};

// Create and start the file transfer manager
let mut file_manager = FileTransferManager::new(config);
let server_addr = file_manager.start_server().await?;

// Send a file to another node
let target_addr = "192.168.1.100:7879".parse()?;
let file_id = file_manager.send_file("path/to/file.dat", target_addr).await?;
```

### Testing File Transfers

The repository includes a test utility to exercise file transfers between nodes:

```bash
# Build and run the file transfer test utility
cargo run --bin test_file_transfer
```

This interactive utility lets you discover other nodes on the network and send files between them. 