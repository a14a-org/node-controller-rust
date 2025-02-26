mod metrics;
mod api;
mod updater;

use anyhow::Result;
use metrics::{CpuCollector, NetworkCollector, StorageCollector, SystemInfoCollector};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ctrlc;
use serde_json::json;
use api::ApiClient;
use log::{info, error, warn, debug};
use std::env;
use std::str::FromStr;
use dotenv::dotenv;
use std::path::PathBuf;
use updater::{UpdateManager, UpdateConfig, UpdateChannel, Version};

const SERVER_UPDATE_INTERVAL: Duration = Duration::from_secs(5);

fn print_separator() {
    println!("\n{}\n", "-".repeat(80));
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    dotenv().ok();
    
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Get API configuration from environment variables
    let api_url = env::var("MONITORING_API_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let api_key = env::var("MONITORING_API_KEY")
        .unwrap_or_else(|_| "dev-api-key".to_string());

    info!("Starting node controller with monitoring API at: {}", api_url);

    // Initialize API client
    let api_client = match ApiClient::new(api_url, api_key) {
        Ok(client) => {
            info!("API client initialized successfully");
            Some(client)
        },
        Err(err) => {
            error!("Failed to initialize API client: {}", err);
            None
        }
    };

    // Initialize the update manager
    let current_version = updater::Version::from_cargo_toml()
        .unwrap_or_else(|_| {
            warn!("Could not determine current version from Cargo.toml, using 0.1.0");
            Version::from_str("0.1.0").unwrap()
        });
    
    info!("Current version: {}", current_version);
    
    // Configure the update manager
    let update_config = UpdateConfig {
        check_interval_mins: env::var("UPDATE_CHECK_INTERVAL_MINS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60), // Default: check every hour
        
        channel: match env::var("UPDATE_CHANNEL").as_deref() {
            Ok("beta") => UpdateChannel::Beta,
            Ok("nightly") => UpdateChannel::Nightly,
            Ok(custom) if !custom.is_empty() => UpdateChannel::Custom(custom.to_string()),
            _ => UpdateChannel::Stable, // Default to stable
        },
        
        auto_update: env::var("AUTO_UPDATE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false), // Default: notify only
        
        repository: env::var("UPDATE_REPOSITORY")
            .unwrap_or_else(|_| "a14a-org/node-controller-rust".to_string()),
            
        update_dir: PathBuf::from(env::var("UPDATE_DIR")
            .unwrap_or_else(|_| "/Library/NodeController/updates".to_string())),
            
        max_backups: env::var("MAX_BACKUPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3), // Default: keep 3 backups
            
        post_update_commands: env::var("POST_UPDATE_COMMANDS")
            .map(|cmds| cmds.split(';').map(ToString::to_string).collect())
            .unwrap_or_default(),
            
        health_check_timeout: Duration::from_secs(
            env::var("HEALTH_CHECK_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30) // Default: 30 seconds
        ),
    };
    
    info!("Update configuration: channel={:?}, auto_update={}, check_interval={}min",
          update_config.channel,
          update_config.auto_update,
          update_config.check_interval_mins);
    
    // Create and start the update manager
    let mut update_manager = UpdateManager::new(update_config, current_version);
    match update_manager.start().await {
        Ok(_) => info!("Update manager started successfully"),
        Err(e) => warn!("Failed to start update manager: {}", e),
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let mut cpu_collector = CpuCollector::new();
    let mut network_collector = NetworkCollector::new();
    let mut storage_collector = StorageCollector::new();
    let mut system_collector = SystemInfoCollector::new();

    // Collect and display initial system information
    if let Ok(system_info) = system_collector.collect() {
        println!("{}", system_info);
        print_separator();
        
        // Send initial full system info to server
        let _initial_payload = json!({
            "type": "system_info",
            "data": system_info,
            "full_update": true
        });
        // TODO: Send initial_payload to server
    }

    // Collection intervals
    let cpu_interval = Duration::from_secs(2);     // CPU every 2 seconds
    let network_interval = Duration::from_secs(5);  // Network every 5 seconds
    let storage_interval = Duration::from_secs(10); // Storage every 10 seconds
    
    let mut last_cpu = Instant::now();
    let mut last_network = Instant::now();
    let mut last_storage = Instant::now();
    let mut last_server_update = Instant::now();

    // Keep track of metrics for server updates
    let mut pending_cpu_metrics = None;
    let mut pending_network_metrics = None;
    let mut pending_storage_metrics = None;
    let mut pending_system_changes = Vec::new();

    println!("Starting metrics collection (Press Ctrl+C to stop)...");
    print_separator();

    while running.load(Ordering::SeqCst) {
        let now = Instant::now();
        let mut updated_any = false;

        // Log the start of each iteration
        debug!("Main loop iteration starting");

        // Collect CPU metrics if interval has elapsed
        if now.duration_since(last_cpu) >= cpu_interval {
            info!("CPU collection interval reached");
            match cpu_collector.collect() {
                Ok(metrics) => {
                    // Print summary
                    println!("CPU Usage: {:.1}% (User: {:.1}%, System: {:.1}%)",
                        metrics.current_load,
                        metrics.user_load,
                        metrics.system_load
                    );
                    println!("Temperature: {:.1}°C (Max: {:.1}°C)",
                        metrics.temperature_main,
                        metrics.temperature_max
                    );
                    if let Some(apple_data) = &metrics.apple_silicon_data {
                        println!("Power: {:.2}W (CPU: {:.2}W, GPU: {:.2}W)",
                            apple_data.power.package_watts,
                            apple_data.power.cpu_watts,
                            apple_data.power.gpu_watts
                        );
                    }
                    pending_cpu_metrics = Some(metrics);
                    last_cpu = now;
                    updated_any = true;
                    info!("CPU metrics collected successfully");
                },
                Err(err) => {
                    error!("Failed to collect CPU metrics: {}", err);
                }
            }
        }

        // Collect Network metrics if interval has elapsed
        if now.duration_since(last_network) >= network_interval {
            info!("Network collection interval reached");
            match network_collector.collect() {
                Ok(metrics) => {
                    print_separator();
                    println!("Network Interfaces:");
                    for metric in &metrics {
                        println!("{}", metric);
                    }
                    pending_network_metrics = Some(metrics);
                    last_network = now;
                    updated_any = true;
                    info!("Network metrics collected successfully");
                },
                Err(err) => {
                    error!("Failed to collect Network metrics: {}", err);
                }
            }
        }

        // Collect Storage metrics if interval has elapsed
        if now.duration_since(last_storage) >= storage_interval {
            info!("Storage collection interval reached");
            match storage_collector.collect() {
                Ok(metrics) => {
                    print_separator();
                    println!("Storage:");
                    println!("\nFilesystems:");
                    for fs in &metrics.filesystem_metrics {
                        println!("{}", fs);
                    }
                    println!("\nDisk I/O:");
                    println!("{}", metrics.io_metrics);
                    pending_storage_metrics = Some(metrics);
                    last_storage = now;
                    updated_any = true;
                    info!("Storage metrics collected successfully");
                },
                Err(err) => {
                    error!("Failed to collect Storage metrics: {}", err);
                }
            }
        }

        // Check for system changes and prepare server update
        if now.duration_since(last_server_update) >= SERVER_UPDATE_INTERVAL {
            // Log the server update check - added for debugging
            info!("SERVER UPDATE INTERVAL REACHED: {} seconds elapsed since last update", 
                  now.duration_since(last_server_update).as_secs());
            
            // Collect latest system info and check for changes
            match system_collector.collect() {
                Ok(system_info) => {
                    info!("System info collected successfully for server update");
                    
                    // If there are changes, add them to pending updates
                    if !system_info.last_update.changed_fields.is_empty() {
                        info!("System changes detected: {:?}", system_info.last_update.changed_fields);
                        pending_system_changes = system_info.last_update.changed_fields.clone();
                    } else {
                        info!("No system changes detected");
                    }

                    // Send metrics to the monitoring API if client is available
                    if let Some(client) = &api_client {
                        info!("Sending metrics to monitoring API...");
                        
                        let send_result = client.send_metrics(
                            &system_info,
                            pending_cpu_metrics.as_ref(),
                            pending_network_metrics.as_ref(),
                            pending_storage_metrics.as_ref(),
                        ).await;
                        
                        match send_result {
                            Ok(_) => info!("Successfully sent metrics to monitoring API"),
                            Err(err) => warn!("Failed to send metrics to monitoring API: {}", err),
                        }
                    } else {
                        // Log if API client is not available - added for debugging
                        warn!("API client is not available for sending metrics");
                        
                        // Prepare the update payload for display
                        let mut update_payload = json!({
                            "timestamp": chrono::Utc::now(),
                            "node_id": system_info.hostname, // Use hostname as node ID
                        });

                        // Add CPU metrics if available
                        if let Some(cpu) = &pending_cpu_metrics {
                            update_payload["cpu"] = json!(cpu);
                        }

                        // Add network metrics if available
                        if let Some(network) = &pending_network_metrics {
                            update_payload["network"] = json!(network);
                        }

                        // Add storage metrics if available
                        if let Some(storage) = &pending_storage_metrics {
                            update_payload["storage"] = json!(storage);
                        }

                        // Add system changes if any
                        if !pending_system_changes.is_empty() {
                            let mut system_update = json!({});
                            for field in &pending_system_changes {
                                match field.as_str() {
                                    "peripherals" => { system_update["peripherals"] = json!(system_info.peripherals); }
                                    "power" => { system_update["power"] = json!(system_info.power); }
                                    "platform" => { 
                                        system_update["platform"] = json!({
                                            "available_memory": system_info.platform.available_memory,
                                            "load_average": system_info.platform.load_average,
                                            "uptime_seconds": system_info.platform.uptime_seconds,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                            update_payload["system_changes"] = system_update;
                        }

                        println!("\nPrepared server update (API client not available):");
                        println!("{}", serde_json::to_string_pretty(&update_payload)?);
                    }

                    // Clear pending updates
                    pending_cpu_metrics = None;
                    pending_network_metrics = None;
                    pending_storage_metrics = None;
                    pending_system_changes.clear();
                    last_server_update = now;
                    updated_any = true;
                    info!("Server update completed");
                },
                Err(err) => {
                    error!("Failed to collect system info for server update: {}", err);
                }
            }
        }

        // Sleep for a short duration to prevent busy waiting
        // Use the shortest of the remaining intervals
        let next_cpu = cpu_interval.saturating_sub(now.duration_since(last_cpu));
        let next_network = network_interval.saturating_sub(now.duration_since(last_network));
        let next_storage = storage_interval.saturating_sub(now.duration_since(last_storage));
        let next_server = SERVER_UPDATE_INTERVAL.saturating_sub(now.duration_since(last_server_update));
        
        // Add debug logging for timing
        debug!(
            "Time to next intervals - CPU: {}s, Network: {}s, Storage: {}s, Server: {}s", 
            next_cpu.as_secs_f32(), 
            next_network.as_secs_f32(), 
            next_storage.as_secs_f32(), 
            next_server.as_secs_f32()
        );
        
        let sleep_duration = next_cpu
            .min(next_network)
            .min(next_storage)
            .min(next_server)
            .min(Duration::from_millis(100));
        
        debug!("Sleeping for {}s", sleep_duration.as_secs_f32());
        
        // Add a safety check to prevent sleeping for 0 duration
        if sleep_duration.as_nanos() == 0 {
            thread::sleep(Duration::from_millis(10));
            warn!("Sleep duration was 0, using 10ms fallback");
        } else {
            thread::sleep(sleep_duration);
        }
        
        // Log if no updates were made in this iteration
        if !updated_any {
            debug!("No metrics were updated in this iteration");
        }
    }

    println!("\nStopping metrics collection...");
    Ok(())
} 