use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub node_id: String,
    pub collected_at: DateTime<Utc>,
    pub interface_name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    #[serde(skip_serializing)]
    pub rx_rate_human: String,
    #[serde(skip_serializing)]
    pub tx_rate_human: String,
    pub interface_info: InterfaceInfo,
}

impl NetworkMetrics {
    fn format_rate(bytes_per_sec: f64) -> String {
        if bytes_per_sec >= 1_000_000_000.0 {
            format!("{:.2} GB/s", bytes_per_sec / 1_000_000_000.0)
        } else if bytes_per_sec >= 1_000_000.0 {
            format!("{:.2} MB/s", bytes_per_sec / 1_000_000.0)
        } else if bytes_per_sec >= 1_000.0 {
            format!("{:.2} KB/s", bytes_per_sec / 1_000.0)
        } else {
            format!("{:.0} B/s", bytes_per_sec)
        }
    }

    pub fn update_human_rates(&mut self) {
        self.rx_rate_human = Self::format_rate(self.rx_bytes_per_sec);
        self.tx_rate_human = Self::format_rate(self.tx_bytes_per_sec);
    }
}

impl fmt::Display for NetworkMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(wifi) = &self.interface_info.wifi_info {
            write!(
                f,
                "{} ({}) - RX: {}, TX: {} - Signal: {} ({} dBm)",
                self.interface_name,
                wifi.ssid,
                self.rx_rate_human,
                self.tx_rate_human,
                wifi.signal_quality(),
                wifi.rssi
            )
        } else {
            write!(
                f,
                "{} ({}) - RX: {}, TX: {}",
                self.interface_name,
                self.interface_info.interface_type,
                self.rx_rate_human,
                self.tx_rate_human
            )
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceInfo {
    pub interface_type: String,
    pub mac: String,
    pub ipv4: String,
    pub ipv6: String,
    pub speed: u64,
    pub status: String,
    pub mtu: u32,
    pub duplex: String,
    pub media_type: String,
    pub supports_ipv6: bool,
    pub wifi_info: Option<WifiInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WifiInfo {
    pub ssid: String,
    pub channel: u32,
    pub rssi: i32,        // Signal strength in dBm
    pub noise: i32,       // Noise level in dBm
    pub tx_rate: u32,     // Current transmission rate in Mbps
    pub auth_type: String, // Authentication type (WPA2, etc.)
}

impl WifiInfo {
    pub fn signal_quality(&self) -> &'static str {
        match self.rssi {
            rssi if rssi >= -50 => "Excellent",
            rssi if rssi >= -60 => "Good",
            rssi if rssi >= -70 => "Fair",
            _ => "Poor",
        }
    }

    pub fn snr(&self) -> i32 {
        self.rssi - self.noise
    }
} 