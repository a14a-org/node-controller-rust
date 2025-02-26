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