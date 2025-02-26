use anyhow::Result;
use chrono::Utc;
use std::process::Command;
use uuid::Uuid;
use std::time::Instant;

use super::types::{StorageMetrics, FilesystemMetric, IoMetrics};

const RATE_SMOOTHING_FACTOR: f64 = 0.3; // Lower = more smoothing

pub struct StorageCollector {
    node_id: String,
    last_io: Option<(u64, u64, Instant)>, // (total_read, total_write, timestamp)
    smoothed_rates: (f64, f64), // (read_rate, write_rate)
}

impl StorageCollector {
    pub fn new() -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            last_io: None,
            smoothed_rates: (0.0, 0.0),
        }
    }

    pub fn collect(&mut self) -> Result<StorageMetrics> {
        let filesystem_metrics = self.collect_filesystem_metrics()?;
        let io_metrics = self.collect_io_metrics()?;
        
        Ok(StorageMetrics {
            node_id: self.node_id.clone(),
            collected_at: Utc::now(),
            filesystem_metrics,
            io_metrics,
        })
    }

    fn collect_filesystem_metrics(&self) -> Result<Vec<FilesystemMetric>> {
        let mut metrics = Vec::new();
        
        // Use df to get filesystem information
        let output = Command::new("df")
            .args(["-k"]) // Output in 1K blocks
            .output()?;
            
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut lines = output_str.lines();
            
            // Skip header line
            lines.next();
            
            for line in lines {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let size = parts[1].parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                    let used = parts[2].parse::<u64>().unwrap_or(0) * 1024;
                    let available = parts[3].parse::<u64>().unwrap_or(0) * 1024;
                    let used_percent = if size > 0 {
                        (used as f64 / size as f64) * 100.0
                    } else {
                        0.0
                    };

                    metrics.push(FilesystemMetric {
                        fs: parts[0].to_string(),
                        mount: parts[5].to_string(),
                        size,
                        used,
                        available,
                        used_percent,
                    });
                }
            }
        }

        // Sort by mount point
        metrics.sort_by(|a, b| a.mount.cmp(&b.mount));
        
        Ok(metrics)
    }

    fn collect_io_metrics(&mut self) -> Result<IoMetrics> {
        // Use iostat to get I/O statistics
        let output = Command::new("iostat")
            .args(["-d", "-c", "1", "1"]) // Display disk statistics once
            .output()?;
            
        let mut total_read = 0u64;
        let mut total_write = 0u64;
        
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = output_str.lines().collect();
            
            // Process each disk's statistics
            for line in lines.iter().skip(3) { // Skip headers
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    // KB/s read and written
                    if let Ok(read) = parts[2].parse::<f64>() {
                        total_read = (read * 1024.0) as u64; // Convert KB to bytes
                    }
                    if let Ok(write) = parts[3].parse::<f64>() {
                        total_write = (write * 1024.0) as u64;
                    }
                }
            }
        }

        let now = Instant::now();
        
        // Calculate rates
        let (read_rate, write_rate) = if let Some((last_read, last_write, last_time)) = self.last_io {
            let time_diff = now.duration_since(last_time).as_secs_f64();
            if time_diff > 0.0 {
                let read_diff = total_read.saturating_sub(last_read) as f64;
                let write_diff = total_write.saturating_sub(last_write) as f64;
                (read_diff / time_diff, write_diff / time_diff)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        // Apply exponential smoothing to rates
        self.smoothed_rates = (
            (1.0 - RATE_SMOOTHING_FACTOR) * self.smoothed_rates.0 + RATE_SMOOTHING_FACTOR * read_rate,
            (1.0 - RATE_SMOOTHING_FACTOR) * self.smoothed_rates.1 + RATE_SMOOTHING_FACTOR * write_rate,
        );

        // Update last I/O values
        self.last_io = Some((total_read, total_write, now));

        let mut metrics = IoMetrics {
            total_read,
            total_write,
            read_bytes_per_sec: self.smoothed_rates.0,
            write_bytes_per_sec: self.smoothed_rates.1,
            read_rate_human: String::new(),
            write_rate_human: String::new(),
        };

        // Update human-readable rates
        metrics.read_rate_human = StorageMetrics::format_rate(metrics.read_bytes_per_sec);
        metrics.write_rate_human = StorageMetrics::format_rate(metrics.write_bytes_per_sec);

        Ok(metrics)
    }
} 