use anyhow::{Result, Context};
use reqwest::{Client, header};
use log::{info, error, debug, warn};
use std::time::{Duration, Instant};
use crate::metrics::cpu::types::CpuMetrics;
use crate::metrics::network::types::NetworkMetrics;
use crate::metrics::storage::types::StorageMetrics;
use crate::metrics::system::types::SystemInfo;
use super::models;
use chrono::Utc;

/// API client for sending metrics to the monitoring API
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: String, api_key: String) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "X-API-Key",
            header::HeaderValue::from_str(&api_key)
                .context("Invalid API key format")?
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url,
        })
    }

    /// Send system metrics to the monitoring API
    pub async fn send_metrics(
        &self,
        system_info: &SystemInfo,
        cpu_metrics: Option<&CpuMetrics>,
        network_metrics: Option<&Vec<NetworkMetrics>>,
        storage_metrics: Option<&StorageMetrics>,
    ) -> Result<()> {
        let metrics = self.build_metrics_payload(
            system_info,
            cpu_metrics,
            network_metrics,
            storage_metrics,
        )?;

        let endpoint = format!("{}/api/v1/metrics", self.base_url);
        debug!("Sending metrics to API: {}", endpoint);
        
        // Log request summary
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let start_time = Instant::now();
        
        // Create a condensed version of the metrics for logging
        let body_summary = format!(
            "{{ system: {}, cpu: {:.1}%, memory: {:.1}MB free, metrics_count: {} }}", 
            system_info.hostname,
            cpu_metrics.map_or(0.0, |cpu| cpu.current_load),
            system_info.platform.available_memory as f64 / 1024.0 / 1024.0,
            // Count how many types of metrics we're sending
            [cpu_metrics.is_some(), network_metrics.is_some(), storage_metrics.is_some()]
                .iter()
                .filter(|&&present| present)
                .count()
        );
        
        // Send the request
        let response_result = self.client
            .post(&endpoint)
            .json(&metrics)
            .send()
            .await;
            
        let duration = start_time.elapsed().as_millis();
        
        match response_result {
            Ok(response) => {
                let status = response.status();
                
                if status.is_success() {
                    // Try to parse the response
                    match response.json::<models::ApiResponse>().await {
                        Ok(api_response) => {
                            info!("[{}] POST - {}ms - {} - {} - {} OK (node: {})", 
                                  timestamp, duration, endpoint, body_summary, 
                                  status.as_u16(), api_response.node);
                            Ok(())
                        },
                        Err(err) => {
                            warn!("[{}] POST - {}ms - {} - {} - {} ERROR (Failed to parse response: {})", 
                                  timestamp, duration, endpoint, body_summary, 
                                  status.as_u16(), err);
                            Err(anyhow::anyhow!("Failed to parse API response: {}", err))
                        }
                    }
                } else {
                    // Get the error text
                    match response.text().await {
                        Ok(error_text) => {
                            error!("[{}] POST - {}ms - {} - {} - {} ERROR ({})", 
                                   timestamp, duration, endpoint, body_summary, 
                                   status.as_u16(), error_text);
                            Err(anyhow::anyhow!("API error ({}): {}", status, error_text))
                        },
                        Err(err) => {
                            error!("[{}] POST - {}ms - {} - {} - {} ERROR (Failed to get error text: {})", 
                                   timestamp, duration, endpoint, body_summary, 
                                   status.as_u16(), err);
                            Err(anyhow::anyhow!("API error ({}): Failed to get error details", status))
                        }
                    }
                }
            },
            Err(err) => {
                error!("[{}] POST - {}ms - {} - {} - REQUEST FAILED ({})", 
                       timestamp, duration, endpoint, body_summary, err);
                Err(anyhow::anyhow!("Failed to send metrics to API: {}", err))
            }
        }
    }

    /// Build the metrics payload from our internal metrics
    fn build_metrics_payload(
        &self,
        system_info: &SystemInfo,
        cpu_metrics: Option<&CpuMetrics>,
        network_metrics: Option<&Vec<NetworkMetrics>>,
        storage_metrics: Option<&StorageMetrics>,
    ) -> Result<models::SystemMetrics> {
        // Create the base system metrics
        let mut metrics = models::SystemMetrics {
            timestamp: chrono::Utc::now(),
            system: models::SystemInfo {
                hostname: system_info.hostname.clone(),
                platform: system_info.platform.os_type.clone(),
                release: system_info.platform.os_version.clone(),
                uptime: system_info.platform.uptime_seconds,
                loadavg: vec![
                    system_info.platform.load_average.0,
                    system_info.platform.load_average.1,
                    system_info.platform.load_average.2,
                ],
                is_apple_silicon: system_info.hardware.model_identifier.contains("Mac") && 
                                system_info.hardware.processor_name.contains("Apple"),
                model: system_info.hardware.model_name.clone(),
            },
            // Initialize with empty values, will be filled in below if available
            cpu: models::CpuInfo {
                info: models::CpuHardwareInfo {
                    manufacturer: "Unknown".to_string(),
                    brand: "Unknown".to_string(),
                    cores: models::CpuCoreCount {
                        physical: 0,
                        logical: 0,
                    },
                    speed: models::CpuSpeed {
                        base: 0.0,
                        max: 0.0,
                        current: None,
                    },
                },
                load: models::CpuLoadInfo {
                    current: 0.0,
                    user: 0.0,
                    system: 0.0,
                    cores: None,
                },
                temperature: None,
            },
            memory: models::MemoryInfo {
                total: system_info.platform.total_memory,
                used: system_info.platform.total_memory - system_info.platform.available_memory,
                active: 0, // We need to add this metric
                available: system_info.platform.available_memory,
                swap: None, // We need to add swap metrics
            },
            gpu: None,
            network: None,
            thermal: None,
            storage: None,
            peripherals: None,
            apple_silicon: None,
        };

        // Add CPU metrics if available
        if let Some(cpu) = cpu_metrics {
            metrics.cpu = models::CpuInfo {
                info: models::CpuHardwareInfo {
                    manufacturer: cpu.manufacturer.clone(),
                    brand: cpu.brand.clone(),
                    cores: models::CpuCoreCount {
                        physical: cpu.physical_cores,
                        logical: cpu.logical_cores,
                    },
                    speed: models::CpuSpeed {
                        base: cpu.base_speed,
                        max: cpu.max_speed,
                        current: None, // We need to add per-core current speeds
                    },
                },
                load: models::CpuLoadInfo {
                    current: cpu.current_load,
                    user: cpu.user_load,
                    system: cpu.system_load,
                    cores: Some(
                        cpu.core_metrics
                            .iter()
                            .map(|(core_id, metrics)| {
                                models::CoreLoadInfo {
                                    number: core_id.parse::<u32>().unwrap_or(0),
                                    load: metrics.load,
                                    user: metrics.user,
                                    system: metrics.system,
                                }
                            })
                            .collect()
                    ),
                },
                temperature: Some(models::CpuTemperatureInfo {
                    main: cpu.temperature_main,
                    cores: None, // We need to add per-core temperatures
                    max: cpu.temperature_max,
                }),
            };

            // Add Apple Silicon data if available
            if let Some(apple_data) = &cpu.apple_silicon_data {
                metrics.apple_silicon = Some(models::AppleSiliconInfo {
                    chip: models::AppleSiliconChip {
                        model: apple_data.chip.clone(),
                        cores: models::AppleSiliconCores {
                            cpu: system_info.hardware.core_count,
                            gpu: 0, // We need to add GPU core count
                            neural_engine: 0, // We need to add Neural Engine core count
                        },
                    },
                    power: models::AppleSiliconPower {
                        cpu_power: apple_data.power.cpu_watts,
                        gpu_power: apple_data.power.gpu_watts,
                        package_power: apple_data.power.package_watts,
                    },
                    thermal: models::AppleSiliconThermal {
                        levels: models::AppleSiliconThermalLevels {
                            cpu: 0, // We need to add thermal levels
                            gpu: 0,
                            io: 0,
                        },
                    },
                });
            }
        }

        // Add network metrics if available
        if let Some(network) = network_metrics {
            let interfaces = network.iter().map(|net| {
                models::NetworkInterface {
                    name: net.interface_name.clone(),
                    r#type: net.interface_info.interface_type.clone(),
                    mac: net.interface_info.mac.clone(),
                    ipv4: net.interface_info.ipv4.clone(),
                    ipv6: net.interface_info.ipv6.clone(),
                    speed: net.interface_info.speed,
                    status: net.interface_info.status.clone(),
                }
            }).collect();

            let stats = network.iter().map(|net| {
                models::NetworkStats {
                    interface: net.interface_name.clone(),
                    rx_sec: net.rx_bytes_per_sec,
                    tx_sec: net.tx_bytes_per_sec,
                    rx_bytes: net.rx_bytes,
                    tx_bytes: net.tx_bytes,
                    errors: net.rx_errors + net.tx_errors,
                }
            }).collect();

            metrics.network = Some(models::NetworkInfo {
                interfaces: Some(interfaces),
                stats: Some(stats),
            });
        }

        // Add storage metrics if available
        if let Some(storage) = storage_metrics {
            let filesystems = storage.filesystem_metrics.iter().map(|fs| {
                models::FilesystemInfo {
                    fs: fs.fs.clone(),
                    r#type: "unknown".to_string(), // We need to add filesystem type
                    size: fs.size,
                    used: fs.used,
                    available: fs.available,
                    mount: fs.mount.clone(),
                }
            }).collect();

            metrics.storage = Some(models::StorageInfo {
                filesystems: Some(filesystems),
                io: Some(models::IoInfo {
                    total_read: storage.io_metrics.total_read,
                    total_write: storage.io_metrics.total_write,
                    read_bytes_per_sec: storage.io_metrics.read_bytes_per_sec,
                    write_bytes_per_sec: storage.io_metrics.write_bytes_per_sec,
                }),
            });
        }

        // Add GPU info from system info
        if !system_info.hardware.gpu_info.is_empty() {
            let gpus = system_info.hardware.gpu_info.iter().map(|gpu| {
                models::GpuInfo {
                    model: gpu.name.clone(),
                    vendor: gpu.vendor.clone(),
                    vram: gpu.memory_size.map(|size| models::GpuVramInfo {
                        total: size,
                        used: 0, // We need to add GPU memory usage
                        free: size, // Assuming all memory is free for now
                    }),
                }
            }).collect();

            metrics.gpu = Some(gpus);
        }

        // Add thermal info if available
        if system_info.power.battery_present {
            let battery = models::BatteryThermal {
                temperature: 0.0, // We need to add battery temperature
                health: system_info.power.battery_health.as_ref()
                    .and_then(|h| h.parse::<f64>().ok())
                    .unwrap_or(100.0),
                cycle_count: system_info.power.battery_cycle_count.unwrap_or(0),
                is_charging: system_info.power.charging,
                voltage: 0.0, // We need to add battery voltage
                percent: system_info.power.battery_capacity.unwrap_or(0) as f64,
            };

            metrics.thermal = Some(models::ThermalInfo {
                chassis: None, // We need to add chassis temperature
                battery: Some(battery),
                fan: None, // We need to add fan speed
                pressure: None, // We need to add pressure
            });
        }

        Ok(metrics)
    }
} 