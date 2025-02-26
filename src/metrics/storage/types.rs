use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub node_id: String,
    pub collected_at: DateTime<Utc>,
    pub filesystem_metrics: Vec<FilesystemMetric>,
    pub io_metrics: IoMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilesystemMetric {
    pub fs: String,
    pub mount: String,
    pub size: u64,
    pub used: u64,
    pub available: u64,
    #[serde(skip_serializing)]
    pub used_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IoMetrics {
    pub total_read: u64,
    pub total_write: u64,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
    #[serde(skip_serializing)]
    pub read_rate_human: String,
    #[serde(skip_serializing)]
    pub write_rate_human: String,
}

impl StorageMetrics {
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    pub fn format_rate(bytes_per_sec: f64) -> String {
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
}

impl fmt::Display for FilesystemMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} used of {} ({:.1}%) - {} available",
            self.mount,
            StorageMetrics::format_size(self.used),
            StorageMetrics::format_size(self.size),
            self.used_percent,
            StorageMetrics::format_size(self.available)
        )
    }
}

impl fmt::Display for IoMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "I/O: Read {} - Write {}",
            self.read_rate_human,
            self.write_rate_human
        )
    }
} 