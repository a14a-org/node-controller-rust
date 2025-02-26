use anyhow::Result;
use log::{info, warn, debug, error};
use node_controller_rust::networking::{NodeDiscovery, NodeInfo, NodeClient, start_grpc_server};
use std::io::{self, BufRead};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Get node name from command line or use default
    let args: Vec<String> = std::env::args().collect();
    let node_name = args.get(1).cloned().unwrap_or_else(|| {
        hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "test-node".to_string())
    });
    
    // Get port from command line or use default
    let port = args.get(2)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(54321);
    
    info!("Starting gRPC test with name: {}", node_name);
    info!("Using port: {}", port);
    
    // Initialize node discovery
    let discovery = Arc::new(NodeDiscovery::new(&node_name, Some(port))?);
    let local_node = discovery.get_local_node();
    
    // Start the gRPC server
    let addr_str = format!("0.0.0.0:{}", port);
    let addr = SocketAddr::from_str(&addr_str)?;
    start_grpc_server(local_node.clone(), addr).await?;
    
    // Start node discovery
    discovery.start().await?;
    
    // Initialize the node client
    let client = Arc::new(NodeClient::new());
    
    // Node list for easier selection
    let discovered_nodes = Arc::new(Mutex::new(Vec::<NodeInfo>::new()));
    let discovery_clone = discovery.clone();
    let nodes_clone = discovered_nodes.clone();
    
    // Background task to update the list of discovered nodes
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let nodes = discovery_clone.get_discovered_nodes();
            let mut node_list = nodes_clone.lock().await;
            *node_list = nodes;
            
            if !node_list.is_empty() {
                debug!("Updated node list: {} nodes", node_list.len());
            }
        }
    });
    
    // Display help
    println!("\nCommands:");
    println!("  list              - List all discovered nodes");
    println!("  ping <id> <msg>   - Send a ping to a node with the given ID and message");
    println!("  health <id>       - Check health of a node with the given ID");
    println!("  quit              - Exit the test utility");
    println!("Press Enter to see the current node list.\n");
    
    // Input handling loop
    let input = io::stdin();
    let client_ref = client.clone();
    let discovered_nodes_ref = discovered_nodes.clone();
    let local_node_ref = local_node.clone();
    
    for line in input.lock().lines() {
        let line = line?;
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        
        match parts.get(0).map(|&s| s) {
            Some("list") | Some("l") | None | Some("") => {
                // List all discovered nodes
                let nodes = discovered_nodes_ref.lock().await;
                if nodes.is_empty() {
                    println!("No nodes discovered yet.");
                } else {
                    println!("\n=== Discovered Nodes ({}) ===", nodes.len());
                    for (i, node) in nodes.iter().enumerate() {
                        println!("{}. {} ({})", i + 1, node.name, node.id);
                        println!("   Address: {}:{}", node.ip, node.port);
                        println!("   Interface Type: {}", node.interface_type);
                        println!("   Capabilities: {}", node.capabilities.join(", "));
                        println!("   Version: {}", node.version);
                        println!();
                    }
                }
            },
            Some("ping") | Some("p") => {
                // Send a ping to a node
                if parts.len() < 3 {
                    println!("Usage: ping <id> <message>");
                    continue;
                }
                
                let id = parts[1];
                let message = parts[2..].join(" ");
                let nodes = discovered_nodes_ref.lock().await;
                
                if let Some(target_node) = nodes.iter().find(|n| n.id.starts_with(id)) {
                    println!("Pinging node {} ({}) with message: {}", target_node.name, target_node.id, message);
                    
                    match client_ref.ping(target_node, &message, &local_node_ref).await {
                        Ok(response) => {
                            println!("\nReceived response:");
                            println!("  From: {} ({})", response.responder_name, response.responder_id);
                            println!("  Message: {}", response.message);
                            println!("  Round trip time: {} ms", response.response_timestamp - response.request_timestamp);
                        },
                        Err(e) => println!("Error: {}", e),
                    }
                } else {
                    println!("No node found with ID starting with '{}'", id);
                }
            },
            Some("health") | Some("h") => {
                // Check health of a node
                if parts.len() < 2 {
                    println!("Usage: health <id>");
                    continue;
                }
                
                let id = parts[1];
                let nodes = discovered_nodes_ref.lock().await;
                
                if let Some(target_node) = nodes.iter().find(|n| n.id.starts_with(id)) {
                    println!("Checking health of node {} ({})", target_node.name, target_node.id);
                    
                    match client_ref.health_check(target_node, &local_node_ref).await {
                        Ok(response) => {
                            println!("\nHealth check response:");
                            println!("  From: {} ({})", response.responder_name, response.responder_id);
                            println!("  Status: {:?}", response.status);
                            
                            if !response.metrics.is_empty() {
                                println!("  Metrics:");
                                for (key, value) in response.metrics {
                                    println!("    {}: {}", key, value);
                                }
                            }
                        },
                        Err(e) => println!("Error: {}", e),
                    }
                } else {
                    println!("No node found with ID starting with '{}'", id);
                }
            },
            Some("quit") | Some("q") | Some("exit") => {
                println!("Exiting...");
                break;
            },
            Some(cmd) => {
                println!("Unknown command: {}", cmd);
                println!("Available commands: list, ping, health, quit");
            },
        }
    }
    
    Ok(())
} 