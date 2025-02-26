use anyhow::Result;
use chrono::Utc;
use std::process::Command;
use uuid::Uuid;
use std::collections::HashMap;
use std::time::Instant;

use super::types::{NetworkMetrics, InterfaceInfo, WifiInfo};

const RATE_SMOOTHING_FACTOR: f64 = 0.3; // Lower = more smoothing

pub struct NetworkCollector {
    node_id: String,
    last_bytes: HashMap<String, (u64, u64, Instant)>, // (rx_bytes, tx_bytes, timestamp)
    smoothed_rates: HashMap<String, (f64, f64)>, // (rx_rate, tx_rate)
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            last_bytes: HashMap::new(),
            smoothed_rates: HashMap::new(),
        }
    }

    pub fn collect(&mut self) -> Result<Vec<NetworkMetrics>> {
        let mut metrics = Vec::new();
        
        // Get interface details using networksetup
        let interfaces = self.get_interface_details()?;
        
        // Get network usage from netstat
        let usage = self.get_network_usage()?;
        let now = Instant::now();
        
        for (name, stats) in usage {
            if let Some(interface_info) = interfaces.get(&name) {
                // Skip inactive interfaces
                if interface_info.status != "active" {
                    continue;
                }

                // Calculate rates
                let (rx_rate, tx_rate) = if let Some((last_rx, last_tx, last_time)) = self.last_bytes.get(&name) {
                    let time_diff = now.duration_since(*last_time).as_secs_f64();
                    if time_diff > 0.0 {
                        let rx_diff = stats.0.saturating_sub(*last_rx) as f64;
                        let tx_diff = stats.1.saturating_sub(*last_tx) as f64;
                        (rx_diff / time_diff, tx_diff / time_diff)
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                };

                // Apply exponential smoothing to rates
                let (smoothed_rx, smoothed_tx) = self.smoothed_rates
                    .entry(name.clone())
                    .and_modify(|(rx, tx)| {
                        *rx = (1.0 - RATE_SMOOTHING_FACTOR) * *rx + RATE_SMOOTHING_FACTOR * rx_rate;
                        *tx = (1.0 - RATE_SMOOTHING_FACTOR) * *tx + RATE_SMOOTHING_FACTOR * tx_rate;
                    })
                    .or_insert((rx_rate, tx_rate));

                // Update last bytes
                self.last_bytes.insert(name.clone(), (stats.0, stats.1, now));

                let mut metric = NetworkMetrics {
                    node_id: self.node_id.clone(),
                    collected_at: Utc::now(),
                    interface_name: name,
                    rx_bytes: stats.0,
                    tx_bytes: stats.1,
                    rx_errors: stats.2,
                    tx_errors: stats.3,
                    rx_bytes_per_sec: *smoothed_rx,
                    tx_bytes_per_sec: *smoothed_tx,
                    rx_rate_human: String::new(),
                    tx_rate_human: String::new(),
                    interface_info: interface_info.clone(),
                };

                // Update human-readable rates
                metric.update_human_rates();
                metrics.push(metric);
            }
        }
        
        // Sort metrics by interface type (Wi-Fi first, then others)
        metrics.sort_by(|a, b| {
            let a_is_wifi = a.interface_info.interface_type == "Wi-Fi";
            let b_is_wifi = b.interface_info.interface_type == "Wi-Fi";
            b_is_wifi.cmp(&a_is_wifi)
        });

        Ok(metrics)
    }

    fn get_network_usage(&self) -> Result<HashMap<String, (u64, u64, u64, u64)>> {
        let mut stats = HashMap::new();
        
        // Use netstat to get network interface statistics
        let output = Command::new("netstat")
            .args(["-ib"])
            .output()?;
            
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut lines = output_str.lines();
            
            // Skip header line
            lines.next();
            
            for line in lines {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let name = parts[0].to_string();
                    let rx_bytes = parts[6].parse::<u64>().unwrap_or(0);
                    let tx_bytes = parts[9].parse::<u64>().unwrap_or(0);
                    let rx_errors = parts[4].parse::<u64>().unwrap_or(0);
                    let tx_errors = parts[7].parse::<u64>().unwrap_or(0);
                    
                    stats.insert(name, (rx_bytes, tx_bytes, rx_errors, tx_errors));
                }
            }
        }
        
        Ok(stats)
    }

    fn get_interface_details(&self) -> Result<HashMap<String, InterfaceInfo>> {
        let mut interfaces = HashMap::new();
        
        // Get list of network services
        let output = Command::new("networksetup")
            .args(["-listallhardwareports"])
            .output()?;
            
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut current_interface = String::new();
            let mut current_mac = String::new();
            let mut current_type = String::new();
            
            for line in output_str.lines() {
                if line.starts_with("Hardware Port:") {
                    current_type = line.split(':').nth(1).unwrap_or("").trim().to_string();
                } else if line.starts_with("Device:") {
                    current_interface = line.split(':').nth(1).unwrap_or("").trim().to_string();
                } else if line.starts_with("Ethernet Address:") {
                    current_mac = line.split(':').nth(1).unwrap_or("").trim().to_string();
                } else if line.is_empty() && !current_interface.is_empty() {
                    // Get IP addresses and other details for this interface
                    if let Ok(info) = self.get_interface_info(&current_interface, &current_type, &current_mac) {
                        interfaces.insert(current_interface.clone(), info);
                    }
                    current_interface.clear();
                    current_mac.clear();
                    current_type.clear();
                }
            }
            
            // Handle the last interface if there was no empty line after it
            if !current_interface.is_empty() {
                if let Ok(info) = self.get_interface_info(&current_interface, &current_type, &current_mac) {
                    interfaces.insert(current_interface.clone(), info);
                }
            }
        }
        
        Ok(interfaces)
    }

    fn get_interface_info(&self, interface: &str, interface_type: &str, mac: &str) -> Result<InterfaceInfo> {
        let mut ipv4 = String::new();
        let mut ipv6 = String::new();
        let mut status = String::from("inactive");
        let mut speed = 0;
        let mut mtu = 0;
        let mut duplex = String::from("unknown");
        let mut media_type = String::from("unknown");
        let mut supports_ipv6 = false;
        let mut wifi_info = None;

        // Get interface status and details using ifconfig
        if let Ok(output) = Command::new("ifconfig")
            .arg(interface)
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("status: active") {
                    status = String::from("active");
                } else if line.contains("inet ") {
                    ipv4 = line.split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                } else if line.contains("inet6 ") && !line.contains("%") {
                    ipv6 = line.split_whitespace()
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    supports_ipv6 = true;
                } else if line.contains("mtu ") {
                    mtu = line.split_whitespace()
                        .nth(1)
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0);
                }
            }
        }

        // Get detailed media info using networksetup
        if let Ok(output) = Command::new("networksetup")
            .args(["-getmedia", interface])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("current") && line.contains("baseT") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        speed = parts[0].parse().unwrap_or(0);
                        if line.contains("full-duplex") {
                            duplex = String::from("full");
                        } else if line.contains("half-duplex") {
                            duplex = String::from("half");
                        }
                        media_type = parts.iter()
                            .find(|&&p| p.contains("baseT"))
                            .unwrap_or(&"unknown")
                            .to_string();
                    }
                }
            }
        }

        // If MTU is still 0, try to get it from sysctl
        if mtu == 0 {
            if let Ok(output) = Command::new("sysctl")
                .args(["-n", &format!("net.inet.tcp.mssdflt")])
                .output()
            {
                if let Ok(mss) = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u32>()
                {
                    mtu = mss + 40; // TCP MSS + IP header (20) + TCP header (20)
                }
            }
        }

        // Get Wi-Fi information if this is a Wi-Fi interface
        if interface_type == "Wi-Fi" && status == "active" {
            // Get SSID
            if let Ok(output) = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
                .args(["-I"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let mut ssid = String::new();
                let mut channel = 0;
                let mut rssi = 0;
                let mut noise = 0;
                let mut tx_rate = 0;
                let mut auth_type = String::from("unknown");

                for line in output_str.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim();
                        let value = parts[1].trim();
                        match key {
                            " SSID" => ssid = value.to_string(),
                            " channel" => {
                                if let Some(ch) = value.split(',').next() {
                                    channel = ch.parse().unwrap_or(0);
                                }
                            },
                            " agrCtlRSSI" => rssi = value.parse().unwrap_or(0),
                            " agrCtlNoise" => noise = value.parse().unwrap_or(0),
                            " lastTxRate" => tx_rate = value.parse().unwrap_or(0),
                            " link auth" => auth_type = value.to_string(),
                            _ => (), // Ignore other fields
                        }
                    }
                }

                if !ssid.is_empty() {
                    wifi_info = Some(WifiInfo {
                        ssid,
                        channel,
                        rssi,
                        noise,
                        tx_rate,
                        auth_type,
                    });
                }
            }
        }

        Ok(InterfaceInfo {
            interface_type: interface_type.to_string(),
            mac: mac.to_string(),
            ipv4,
            ipv6,
            speed,
            status,
            mtu,
            duplex,
            media_type,
            supports_ipv6,
            wifi_info,
        })
    }
} 