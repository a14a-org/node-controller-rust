# Testing Node Discovery Functionality

This guide will help you test the node discovery functionality that allows nodes to find each other automatically over the network.

## Requirements

- Two or more machines connected to the same network
- Each machine should have the node-controller-rust repository cloned
- Rust and Cargo installed on all machines

## Testing Process

### 1. Prepare the Environment

Make sure both machines are connected to the same network. Ideally, they should have a direct Thunderbolt or Ethernet connection between them for the best test results.

### 2. Running the Test Utility

You can run the test utility in one of two ways:

#### Using the Helper Script

The simplest way is to use the provided helper script:

```bash
./test_discovery.sh [node-name] [log-level]
```

Example:
```bash
# Run with custom node name
./test_discovery.sh my-node-1

# Run with custom node name and debug logging
./test_discovery.sh my-node-1 debug
```

#### Manual Run

Alternatively, you can build and run the test binary directly:

```bash
# Build the test binary
cargo build --bin test_discovery

# Run with default settings
RUST_LOG=info ./target/debug/test_discovery

# Run with custom node name
RUST_LOG=info ./target/debug/test_discovery my-custom-node

# Run with custom port
RUST_LOG=info ./target/debug/test_discovery my-custom-node 55555
```

### 3. What to Look For

When you run the utility, it will:

1. Display all detected network interfaces and their types
2. Show which interface is selected for node communication
3. Initialize and start the discovery service
4. Display information about the local node
5. Continuously scan for other nodes on the network
6. Display discovered nodes every 10 seconds or when you press Enter

### 4. Verifying Discovery Works

To verify that the discovery system is working correctly:

1. Run the test utility on at least two different machines
2. Each machine should discover the other machine(s) within ~10 seconds
3. The discovered nodes should display the correct information (name, IP, interface type)
4. The connection should prioritize Thunderbolt interfaces if available

### 5. Testing Interface Priority

To verify that interface priority works:

1. If you have a Thunderbolt connection between machines, it should be selected as the best interface
2. If you disconnect the Thunderbolt connection, it should fall back to Ethernet
3. If you then disconnect Ethernet, it should fall back to WiFi

### 6. Troubleshooting

If nodes don't discover each other:

1. Check that both machines are on the same network
2. Verify that mDNS traffic is allowed (port 5353 UDP)
3. Check if any firewalls are blocking multicast traffic
4. Try increasing log verbosity to debug: `RUST_LOG=debug ./test_discovery.sh my-node`
5. Check if the interface detection correctly identifies your network interfaces

## Advanced Testing

### Testing with Custom Node Names

You can use custom node names to easily identify different machines:

```bash
# On Machine 1
./test_discovery.sh node-alpha

# On Machine 2
./test_discovery.sh node-beta

# On Machine 3
./test_discovery.sh node-gamma
```

### Testing with Custom Ports

If the default port (54321) is blocked or in use, you can specify a custom port:

```bash
# Run with custom port
./test_discovery.sh my-node 55555
```

### Testing Interface Detection

To verify that the interface detection correctly identifies Thunderbolt bridges:

1. Create a Thunderbolt bridge between two Macs
2. Run the test utility on both machines
3. Verify that the Thunderbolt interface is identified correctly and given highest priority 