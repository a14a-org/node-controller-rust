use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use node_controller_rust::networking::{
    FileTransferConfig, FileTransferManager, NodeDiscovery, NodeInfo, TransferStatus,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    info!("---- High-Performance File Transfer Test Utility ----");
    info!("This utility demonstrates the optimized TCP file transfer system");
    info!("which serves as a fallback when RDMA is not available.");
    info!("");

    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();
    let node_name = if args.len() > 1 {
        args[1].clone()
    } else {
        hostname::get()
            .expect("Failed to get hostname")
            .to_string_lossy()
            .to_string()
    };

    info!("Node name: {}", node_name);

    // Set up node discovery
    let discovery = Arc::new(NodeDiscovery::new(&node_name, None)?);
    
    // Start discovery service
    discovery.start().await?;
    
    // Create a directory for received files
    let receive_dir = std::env::temp_dir().join("node_controller_files");
    std::fs::create_dir_all(&receive_dir)?;
    info!("Files will be received in: {}", receive_dir.display());

    // Set up the file transfer manager with progress reporting
    let transfer_config = FileTransferConfig {
        chunk_size: 1024 * 1024, // 1MB chunks
        port: 7879,              // Default port
        receive_dir,
        progress_callback: Some(Arc::new(report_progress)),
        concurrent_streams: 4,   // Use 4 parallel streams
    };

    // Create and start file transfer manager
    let mut file_manager = FileTransferManager::new(transfer_config);
    let server_addr = file_manager.start_server().await?;
    info!("File transfer server started on {}", server_addr);

    // Discovered nodes list
    let nodes = Arc::new(Mutex::new(Vec::<NodeInfo>::new()));
    let nodes_clone = nodes.clone();

    // Spawn a background task to update the list of nodes
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let discovered = discovery.get_nodes().await;
            let mut nodes_guard = nodes_clone.lock().await;
            *nodes_guard = discovered;
        }
    });

    // Display help
    print_help();

    // Main input loop
    let mut input = String::new();
    loop {
        input.clear();
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush()?;
        
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0].to_lowercase();

        match command.as_str() {
            "help" | "h" => {
                print_help();
            }
            "list" | "ls" => {
                let nodes_guard = nodes.lock().await;
                if nodes_guard.is_empty() {
                    info!("No nodes discovered yet");
                } else {
                    info!("Discovered nodes:");
                    for (i, node) in nodes_guard.iter().enumerate() {
                        info!("  {}. {} ({})", i + 1, node.name, node.id);
                        info!("     Address: {}", node.address);
                    }
                }
            }
            "send" => {
                if parts.len() < 3 {
                    error!("Usage: send <node_id> <file_path>");
                    continue;
                }
                
                let node_id = parts[1];
                let file_path = parts[2];
                
                // Find the node
                let target_node = {
                    let nodes_guard = nodes.lock().await;
                    nodes_guard
                        .iter()
                        .find(|n| n.id.starts_with(node_id) || n.name == node_id)
                        .cloned()
                };
                
                match target_node {
                    Some(node) => {
                        info!("Sending file to {} ({})", node.name, node.id);
                        
                        // Construct target address for file transfer
                        let target_addr = node.address
                            .replace("grpc://", "")  // Remove grpc:// prefix if present
                            .parse()?;
                            
                        // Send the file
                        match file_manager.send_file(file_path, target_addr).await {
                            Ok(transfer_id) => {
                                info!("Transfer initiated with ID: {}", transfer_id);
                            }
                            Err(e) => {
                                error!("Failed to send file: {}", e);
                            }
                        }
                    }
                    None => {
                        error!("Node not found: {}", node_id);
                    }
                }
            }
            "status" => {
                info!("File transfer server is running on {}", server_addr);
                info!("Receive directory: {}", file_manager.server_address().await.unwrap());
            }
            "exit" | "quit" | "q" => {
                info!("Shutting down...");
                // Stop the file transfer server
                file_manager.stop_server().await;
                break;
            }
            _ => {
                error!("Unknown command: {}", command);
                print_help();
            }
        }
    }

    Ok(())
}

fn print_help() {
    info!("\nAvailable commands:");
    info!("  help, h            - Show this help");
    info!("  list, ls           - List discovered nodes");
    info!("  send <node> <file> - Send file to node (use node ID or name)");
    info!("  status             - Show file transfer server status");
    info!("  exit, quit, q      - Exit the application");
    info!("");
}

// Progress reporting callback
fn report_progress(status: TransferStatus) {
    match status {
        TransferStatus::Started { file_id, file_name, file_size } => {
            let size_mb = file_size as f64 / (1024.0 * 1024.0);
            info!("⬆️ Transfer started: {} ({:.2} MB)", file_name, size_mb);
        }
        TransferStatus::Progress { file_id, bytes_transferred, total_bytes, percent_complete } => {
            // Only log every 10% to avoid log spam
            if percent_complete.round() % 10.0 == 0.0 {
                let transferred_mb = bytes_transferred as f64 / (1024.0 * 1024.0);
                let total_mb = total_bytes as f64 / (1024.0 * 1024.0);
                info!(
                    "📊 Transfer progress: {:.1}% ({:.2}/{:.2} MB)",
                    percent_complete, transferred_mb, total_mb
                );
            }
        }
        TransferStatus::Completed { file_id, bytes_transferred, elapsed_seconds, throughput_mbps } => {
            let size_mb = bytes_transferred as f64 / (1024.0 * 1024.0);
            info!(
                "✅ Transfer completed: {:.2} MB in {:.2}s ({:.2} MB/s)",
                size_mb, elapsed_seconds, throughput_mbps
            );
        }
        TransferStatus::Failed { file_id, error } => {
            error!("❌ Transfer failed: {}", error);
        }
    }
} 