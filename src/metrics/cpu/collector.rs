use anyhow::Result;
use chrono::Utc;
use sysinfo::System;
use std::collections::HashMap;
use std::process::Command;
use uuid::Uuid;

use super::types::{CpuMetrics, CoreMetrics, AppleSiliconData, PowerMetrics, ThermalMetrics};

pub struct CpuCollector {
    sys: System,
    node_id: String,
}

impl CpuCollector {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu(); // Initial refresh to get baseline CPU metrics
        std::thread::sleep(std::time::Duration::from_millis(100)); // Wait for initial sample
        
        Self {
            sys,
            node_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn collect(&mut self) -> Result<CpuMetrics> {
        // Get CPU metrics with proper sampling
        self.sys.refresh_cpu();
        std::thread::sleep(std::time::Duration::from_millis(500)); // Wait for meaningful CPU load data
        self.sys.refresh_cpu();

        let cpus = self.sys.cpus();
        let global_cpu = &cpus[0]; // Use first CPU for global metrics
        
        // Collect core-specific metrics
        let mut core_metrics = HashMap::new();
        for (i, cpu) in cpus.iter().enumerate() {
            // Note: Using approximation for user/system split since sysinfo doesn't provide this detail
            // Based on typical macOS workload distribution: ~70% user, ~30% system
            let usage = cpu.cpu_usage() as f64;
            core_metrics.insert(format!("core{}", i), CoreMetrics {
                load: usage,
                user: usage * 0.7,
                system: usage * 0.3,
            });
        }

        // Try to collect Apple Silicon specific data
        let apple_silicon_data = self.collect_apple_silicon_data()?;

        // Get main temperature from Apple Silicon data if available
        let (temp_main, temp_max) = if let Some(data) = &apple_silicon_data {
            (data.thermal.cpu_die, data.thermal.cpu_die.max(data.thermal.gpu_die))
        } else {
            (0.0, 0.0)
        };

        let usage = global_cpu.cpu_usage() as f64;
        Ok(CpuMetrics {
            node_id: self.node_id.clone(),
            collected_at: Utc::now(),
            manufacturer: String::from("Apple Inc."),
            brand: global_cpu.brand().to_string(),
            physical_cores: self.sys.physical_core_count().unwrap_or(0) as u32,
            logical_cores: self.sys.cpus().len() as u32,
            base_speed: global_cpu.frequency() as f64,
            max_speed: global_cpu.frequency() as f64,
            current_load: usage,
            user_load: usage * 0.7,   // Approximate user/system split
            system_load: usage * 0.3,  // Could be improved with process-specific metrics
            temperature_main: temp_main,
            temperature_max: temp_max,
            core_metrics,
            apple_silicon_data,
        })
    }

    fn collect_apple_silicon_data(&self) -> Result<Option<AppleSiliconData>> {
        #[cfg(target_os = "macos")]
        {
            // First, get chip information
            let chip = self.detect_apple_silicon_chip()?;
            
            // For testing the auto-updater, return dummy data instead of requiring sudo
            return Ok(Some(AppleSiliconData {
                chip,
                power: PowerMetrics {
                    package_watts: 2.5,
                    cpu_watts: 1.8,
                    gpu_watts: 0.7,
                    ane_watts: 0.0,
                },
                thermal: ThermalMetrics {
                    cpu_die: 40.0,
                    gpu_die: 38.5,
                    efficiency_cores: 39.0,
                    performance_cores: 41.0,
                },
            }));
            
            /*
            // Get power and thermal metrics using powermetrics with all relevant samplers
            let output = Command::new("sudo")
                .args([
                    "powermetrics",
                    "-s", "cpu_power,gpu_power,thermal",
                    "--show-process-energy",
                    "--show-process-coalition",
                    "-i", "200",  // 200ms interval
                    "-n", "1",    // Single sample
                    "--show-extra-power-info"  // For detailed power info
                ])
                .output()?;

            let mut cpu_watts = 0.0;
            let mut gpu_watts = 0.0;
            let mut cpu_temp = 0.0;
            let mut gpu_temp = 0.0;
            let mut perf_cores_temp = 0.0;
            let mut eff_cores_temp = 0.0;
            let mut package_watts = 0.0;
            let mut e_cluster_freq = 0.0;
            let mut p_cluster_freq = 0.0;
            let mut is_throttling = false;

            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("CPU Power") {
                        if let Some(value) = line.split(':').nth(1) {
                            if let Some(num) = value.trim().split_whitespace().next() {
                                // Convert milliwatts to watts
                                cpu_watts = num.parse::<f64>().unwrap_or(0.0) / 1000.0;
                            }
                        }
                    } else if line.contains("GPU Power") {
                        if let Some(value) = line.split(':').nth(1) {
                            if let Some(num) = value.trim().split_whitespace().next() {
                                // Convert milliwatts to watts
                                gpu_watts = num.parse::<f64>().unwrap_or(0.0) / 1000.0;
                            }
                        }
                    } else if line.contains("package_power:") {
                        if let Some(value) = line.split(':').nth(1) {
                            if let Some(num) = value.trim().split_whitespace().next() {
                                // Convert milliwatts to watts
                                package_watts = num.parse::<f64>().unwrap_or(0.0) / 1000.0;
                            }
                        }
                    } else if line.contains("E-Cluster HW active frequency:") {
                        if let Some(value) = line.split(':').nth(1) {
                            if let Some(num) = value.trim().split_whitespace().next() {
                                e_cluster_freq = num.parse::<f64>().unwrap_or(0.0);
                            }
                        }
                    } else if line.contains("P1-Cluster HW active frequency:") {
                        if let Some(value) = line.split(':').nth(1) {
                            if let Some(num) = value.trim().split_whitespace().next() {
                                p_cluster_freq = num.parse::<f64>().unwrap_or(0.0);
                            }
                        }
                    } else if line.contains("throttle:") {
                        is_throttling = line.contains("yes");
                    }
                }

                // If package power is available, use it, otherwise sum CPU and GPU
                if package_watts > 0.0 {
                    cpu_watts = package_watts - gpu_watts;
                }

                // Calculate temperatures based on multiple factors
                let base_temp = 30.0;
                
                // Power contribution (approximate based on typical thermal profiles)
                let power_temp = package_watts * 1.5;
                
                // Frequency contribution
                let e_cluster_temp = (e_cluster_freq / 2800.0) * 8.0; // 2.8 GHz = ~8°C
                let p_cluster_temp = (p_cluster_freq / 4500.0) * 15.0; // 4.5 GHz = ~15°C
                
                // Throttling adds heat
                let throttle_temp = if is_throttling { 5.0 } else { 0.0 };
                
                // Calculate final temps
                cpu_temp = base_temp + power_temp + p_cluster_temp + throttle_temp;
                gpu_temp = base_temp + (gpu_watts * 2.0) + throttle_temp; // GPU typically runs cooler
                perf_cores_temp = base_temp + power_temp + p_cluster_temp + throttle_temp;
                eff_cores_temp = base_temp + power_temp + e_cluster_temp + throttle_temp;
            }

            return Ok(Some(AppleSiliconData {
                chip,
                power: PowerMetrics {
                    package_watts,
                    cpu_watts,
                    gpu_watts,
                    ane_watts: 0.0, // Not directly available
                },
                thermal: ThermalMetrics {
                    cpu_die: cpu_temp,
                    gpu_die: gpu_temp,
                    efficiency_cores: eff_cores_temp,
                    performance_cores: perf_cores_temp,
                },
            }));
            */
        }
        #[cfg(not(target_os = "macos"))]
        Ok(None)
    }

    fn detect_apple_silicon_chip(&self) -> Result<String> {
        // Try sysctl first for most accurate information
        if let Ok(output) = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            let chip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !chip.is_empty() {
                return Ok(chip);
            }
        }

        // Try system_profiler as fallback
        if let Ok(output) = Command::new("system_profiler")
            .args(["SPHardwareDataType"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("Chip:") {
                    if let Some(chip) = line.split(':').nth(1) {
                        return Ok(chip.trim().to_string());
                    }
                }
            }
        }

        Ok(String::from("Apple Silicon"))
    }
} 