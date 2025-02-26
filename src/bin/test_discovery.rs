use anyhow::Result;
use dotenv::dotenv;
use log::{info, warn, error, debug};
use node_controller_rust::networking::{NodeDiscovery, NetworkInterface};
use std::env;
use std::time::Duration;
use std::io;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    dotenv().ok();
    
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    println!("=== Node Discovery Test Utility ===");
    println!("This utility will help test the node discovery functionality.");
    println!("Run it on multiple machines to see them discover each other.");
    println!();
    
    // Get node name from args or env, with fallback to hostname
    let hostname = if let Some(arg) = std::env::args().nth(1) {
        arg
    } else {
        env::var("NODE_NAME").ok().unwrap_or_else(|| {
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "test-node".to_string())
        })
    };
    
    // Get custom port if specified
    let port_str = std::env::args().nth(2);
    let port = port_str
        .as_ref()
        .and_then(|p| p.parse::<u16>().ok())
        .or_else(|| env::var("DISCOVERY_PORT").ok().and_then(|p| p.parse().ok()));
    
    println!("Starting node discovery with name: {} (port: {})", 
             hostname, port.unwrap_or(54321));
    
    // First, detect and display network interfaces
    println!("\n=== Network Interfaces ===");
    match node_controller_rust::networking::interface::discover_interfaces() {
        Ok(interfaces) => {
            for (i, interface) in interfaces.iter().enumerate() {
                println!("{}: {} - {} ({:?}) [Priority: {}]", 
                         i+1, interface.name, interface.ip, 
                         interface.interface_type, interface.priority);
            }
            
            // Show which interface would be selected for node communication
            if let Ok(best_interface) = node_controller_rust::networking::interface::get_best_interface() {
                println!("\n✓ Best interface for node communication: {} - {} ({:?})", 
                       best_interface.name, best_interface.ip, best_interface.interface_type);
            }
        },
        Err(e) => {
            println!("❌ Failed to discover network interfaces: {}", e);
            return Err(e);
        }
    }
    
    // Initialize node discovery
    let discovery = match NodeDiscovery::new(&hostname, port) {
        Ok(d) => {
            println!("\n✓ Successfully initialized discovery service");
            d
        },
        Err(e) => {
            println!("❌ Failed to initialize discovery service: {}", e);
            return Err(e);
        }
    };
    
    // Start the discovery service
    match discovery.start().await {
        Ok(_) => println!("✓ Successfully started discovery service"),
        Err(e) => {
            println!("❌ Failed to start discovery service: {}", e);
            return Err(e);
        }
    }
    
    // Get info about local node
    let local_node = discovery.get_local_node();
    println!("\n=== Local Node Info ===");
    println!("ID: {}", local_node.id);
    println!("Name: {}", local_node.name);
    println!("Address: {}:{}", local_node.ip, local_node.port);
    println!("Interface Type: {}", local_node.interface_type);
    println!("Capabilities: {}", local_node.capabilities.join(", "));
    println!("Version: {}", local_node.version);
    
    println!("\n=== Discovery Running ===");
    println!("Press Ctrl+C to exit or Enter to refresh the node list...");
    
    // Start asynchronous input handling and node monitoring
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    
    // Handle user input in a separate task
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    if let Err(_) = tx_clone.send(()).await {
                        break;
                    }
                },
                Err(e) => {
                    error!("Error reading input: {}", e);
                    break;
                }
            }
        }
    });
    
    // Setup automatic refresh every 10 seconds
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            let _ = tx.send(()).await;
        }
    });
    
    // Main loop to display discovered nodes
    loop {
        // Wait for user input or timer
        match rx.recv().await {
            Some(_) => {
                let nodes = discovery.get_discovered_nodes();
                println!("\n=== Discovered Nodes ({}) ===", nodes.len());
                
                if nodes.is_empty() {
                    println!("No nodes discovered yet. Make sure other instances are running on the network.");
                } else {
                    for (i, node) in nodes.iter().enumerate() {
                        println!("{}: {} ({})", i+1, node.name, node.id);
                        println!("   Address: {}:{}", node.ip, node.port);
                        println!("   Interface Type: {}", node.interface_type);
                        println!("   Capabilities: {}", node.capabilities.join(", "));
                        println!("   Version: {}", node.version);
                        println!();
                    }
                }
                println!("Press Enter to refresh the node list...");
            },
            None => break,
        }
    }
    
    Ok(())
} 