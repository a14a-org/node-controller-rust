use anyhow::Result;
use chrono::Utc;
use std::process::Command;
use std::str;
use std::time::Duration;

use super::types::{SystemInfo, PlatformInfo, HardwareInfo, PeripheralDevice, DisplayInfo, PowerInfo, GpuInfo, UpdateTracker};

const FULL_UPDATE_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes
const PERIPHERAL_CHECK_INTERVAL: Duration = Duration::from_secs(5); // 5 seconds
const POWER_CHECK_INTERVAL: Duration = Duration::from_secs(30); // 30 seconds

pub struct SystemInfoCollector {
    last_info: Option<SystemInfo>,
}

impl SystemInfoCollector {
    pub fn new() -> Self {
        Self {
            last_info: None,
        }
    }

    pub fn collect(&mut self) -> Result<SystemInfo> {
        let now = Utc::now();
        let mut info = if let Some(last_info) = &self.last_info {
            // Check if we need a full update
            if now.signed_duration_since(last_info.last_update.last_full_update) >= chrono::Duration::from_std(FULL_UPDATE_INTERVAL)? {
                self.collect_full_info()?
            } else {
                last_info.clone()
            }
        } else {
            self.collect_full_info()?
        };

        // Update timestamps
        info.collected_at = now;
        
        // Check for peripheral changes if needed
        if now.signed_duration_since(info.last_update.last_peripheral_check) >= chrono::Duration::from_std(PERIPHERAL_CHECK_INTERVAL)? {
            let new_peripherals = self.collect_peripherals()?;
            if info.peripherals != new_peripherals {
                info.last_update.changed_fields.push("peripherals".to_string());
                info.peripherals = new_peripherals;
            }
            info.last_update.last_peripheral_check = now;
        }

        // Check power status if needed
        if now.signed_duration_since(info.last_update.last_power_check) >= chrono::Duration::from_std(POWER_CHECK_INTERVAL)? {
            let new_power = self.collect_power_info()?;
            if info.power != new_power {
                info.last_update.changed_fields.push("power".to_string());
                info.power = new_power;
            }
            info.last_update.last_power_check = now;
        }

        // Update dynamic platform info
        let new_platform = self.collect_platform_info()?;
        if info.platform.available_memory != new_platform.available_memory ||
           info.platform.load_average != new_platform.load_average {
            info.last_update.changed_fields.push("platform".to_string());
            info.platform.available_memory = new_platform.available_memory;
            info.platform.load_average = new_platform.load_average;
            info.platform.uptime_seconds = new_platform.uptime_seconds;
        }

        self.last_info = Some(info.clone());
        Ok(info)
    }

    fn collect_full_info(&self) -> Result<SystemInfo> {
        Ok(SystemInfo {
            collected_at: Utc::now(),
            hostname: self.get_hostname()?,
            platform: self.collect_platform_info()?,
            hardware: self.collect_hardware_info()?,
            peripherals: self.collect_peripherals()?,
            displays: self.collect_displays()?,
            power: self.collect_power_info()?,
            last_update: UpdateTracker {
                last_full_update: Utc::now(),
                last_peripheral_check: Utc::now(),
                last_power_check: Utc::now(),
                changed_fields: vec!["full_update".to_string()],
            },
        })
    }

    fn get_hostname(&self) -> Result<String> {
        let output = Command::new("hostname").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn collect_platform_info(&self) -> Result<PlatformInfo> {
        // Get OS information using sw_vers on macOS
        let os_type = String::from_utf8_lossy(&Command::new("sw_vers").arg("-productName").output()?.stdout).trim().to_string();
        let os_version = String::from_utf8_lossy(&Command::new("sw_vers").arg("-productVersion").output()?.stdout).trim().to_string();
        
        // Get kernel version and architecture
        let kernel_version = String::from_utf8_lossy(&Command::new("uname").arg("-v").output()?.stdout).trim().to_string();
        let architecture = String::from_utf8_lossy(&Command::new("uname").arg("-m").output()?.stdout).trim().to_string();
        
        // Get boot time and uptime
        let uptime_output = Command::new("sysctl").arg("-n").arg("kern.boottime").output()?;
        let _uptime_str = String::from_utf8_lossy(&uptime_output.stdout);
        let boot_time = Utc::now(); // Fallback
        let uptime_seconds = String::from_utf8_lossy(&Command::new("sysctl").arg("-n").arg("kern.boottime").output()?.stdout)
            .split_whitespace()
            .nth(3)
            .and_then(|s| s.trim_matches(',').parse::<u64>().ok())
            .unwrap_or(0);

        // Get memory information
        let total_memory = if let Ok(pages) = String::from_utf8_lossy(&Command::new("sysctl").arg("-n").arg("hw.memsize").output()?.stdout)
            .trim()
            .parse::<u64>() {
            pages
        } else {
            0
        };

        let available_memory = String::from_utf8_lossy(&Command::new("vm_stat").output()?.stdout)
            .lines()
            .find(|line| line.contains("Pages free"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.trim().trim_matches('.').parse::<u64>().ok())
            .map(|pages| pages * 4096) // Convert pages to bytes
            .unwrap_or(0);

        // Get load average
        let loadavg_output = Command::new("sysctl").arg("-n").arg("vm.loadavg").output()?;
        let loadavg_str = String::from_utf8_lossy(&loadavg_output.stdout);
        let load_average = if let Some(loads) = loadavg_str.split_whitespace().collect::<Vec<_>>().get(1..4) {
            (
                loads[0].parse::<f64>().unwrap_or(0.0),
                loads[1].parse::<f64>().unwrap_or(0.0),
                loads[2].parse::<f64>().unwrap_or(0.0),
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        Ok(PlatformInfo {
            os_type,
            os_version,
            kernel_version,
            architecture,
            boot_time,
            uptime_seconds,
            available_memory,
            total_memory,
            load_average,
        })
    }

    fn collect_hardware_info(&self) -> Result<HardwareInfo> {
        let output = Command::new("system_profiler")
            .args(["SPHardwareDataType"])
            .output()?;
        let info = String::from_utf8_lossy(&output.stdout);
        
        let mut model_name = String::new();
        let mut model_identifier = String::new();
        let mut processor_name = String::new();
        let mut processor_speed = String::new();
        let mut processor_count = 0;
        let mut core_count = 0;
        let mut memory_size = 0u64;
        let memory_type = String::from("LPDDR5");
        let mut serial_number = None;

        for line in info.lines() {
            let line = line.trim();
            if line.starts_with("Model Name:") {
                model_name = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("Model Identifier:") {
                model_identifier = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("Chip:") {
                processor_name = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("Processor Speed:") {
                processor_speed = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("Memory:") {
                if let Some(mem_str) = line.split(':').nth(1) {
                    if let Some(gb_str) = mem_str.trim().split_whitespace().next() {
                        if let Ok(gb) = gb_str.parse::<u64>() {
                            memory_size = gb * 1024 * 1024 * 1024;
                        }
                    }
                }
            } else if line.starts_with("Serial Number") {
                serial_number = line.split(':').nth(1).map(|s| s.trim().to_string());
            }
        }

        // Get CPU core information
        if let Ok(count) = String::from_utf8_lossy(&Command::new("sysctl").arg("-n").arg("hw.ncpu").output()?.stdout)
            .trim()
            .parse::<u32>() {
            processor_count = count;
        }

        if let Ok(count) = String::from_utf8_lossy(&Command::new("sysctl").arg("-n").arg("hw.physicalcpu").output()?.stdout)
            .trim()
            .parse::<u32>() {
            core_count = count;
        }

        // Collect GPU information
        let gpu_info = self.collect_gpu_info()?;

        Ok(HardwareInfo {
            model_name,
            model_identifier,
            processor_name,
            processor_speed,
            processor_count,
            core_count,
            memory_size,
            memory_type,
            gpu_info,
            serial_number,
        })
    }

    fn collect_gpu_info(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();
        
        let output = Command::new("system_profiler")
            .args(["SPDisplaysDataType"])
            .output()?;
        let info = String::from_utf8_lossy(&output.stdout);
        
        let mut current_gpu: Option<GpuInfo> = None;
        
        for line in info.lines() {
            let line = line.trim();
            if line.contains("Chipset Model:") {
                if let Some(gpu) = current_gpu.take() {
                    gpus.push(gpu);
                }
                current_gpu = Some(GpuInfo {
                    name: line.split(':').nth(1).unwrap_or("").trim().to_string(),
                    vendor: String::new(),
                    memory_size: None,
                    device_id: String::new(),
                });
            } else if let Some(gpu) = &mut current_gpu {
                if line.contains("Vendor:") {
                    gpu.vendor = line.split(':').nth(1).unwrap_or("").trim().to_string();
                } else if line.contains("VRAM") {
                    if let Some(mem_str) = line.split(':').nth(1) {
                        if let Some(mb_str) = mem_str.trim().split_whitespace().next() {
                            if let Ok(mb) = mb_str.parse::<u64>() {
                                gpu.memory_size = Some(mb * 1024 * 1024);
                            }
                        }
                    }
                } else if line.contains("Device ID:") {
                    gpu.device_id = line.split(':').nth(1).unwrap_or("").trim().to_string();
                }
            }
        }

        if let Some(gpu) = current_gpu {
            gpus.push(gpu);
        }

        Ok(gpus)
    }

    fn collect_displays(&self) -> Result<Vec<DisplayInfo>> {
        let mut displays = Vec::new();
        
        let output = Command::new("system_profiler")
            .args(["SPDisplaysDataType"])
            .output()?;
        let info = String::from_utf8_lossy(&output.stdout);
        
        let mut current_display: Option<DisplayInfo> = None;
        
        for line in info.lines() {
            let line = line.trim();
            if line.ends_with(":") && !line.contains("Displays") {
                if let Some(display) = current_display.take() {
                    displays.push(display);
                }
                current_display = Some(DisplayInfo {
                    name: line.trim_end_matches(':').to_string(),
                    resolution: (0, 0),
                    refresh_rate: 0.0,
                    is_builtin: line.contains("Built-in"),
                    serial_number: None,
                    technology: String::new(),
                });
            } else if let Some(display) = &mut current_display {
                if line.contains("Resolution:") {
                    if let Some(res_str) = line.split(':').nth(1) {
                        let parts: Vec<&str> = res_str.split('x').collect();
                        if parts.len() == 2 {
                            display.resolution = (
                                parts[0].trim().parse().unwrap_or(0),
                                parts[1].trim().parse().unwrap_or(0),
                            );
                        }
                    }
                } else if line.contains("Refresh Rate:") {
                    if let Some(rate_str) = line.split(':').nth(1) {
                        if let Some(rate) = rate_str.trim().split_whitespace().next() {
                            display.refresh_rate = rate.parse().unwrap_or(0.0);
                        }
                    }
                } else if line.contains("Display Type:") {
                    display.technology = line.split(':').nth(1).unwrap_or("").trim().to_string();
                } else if line.contains("Serial Number:") {
                    display.serial_number = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
                }
            }
        }

        if let Some(display) = current_display {
            displays.push(display);
        }

        Ok(displays)
    }

    fn collect_peripherals(&self) -> Result<Vec<PeripheralDevice>> {
        let mut devices = Vec::new();

        // Get USB devices
        let output = Command::new("system_profiler")
            .args(["SPUSBDataType"])
            .output()?;
        let info = String::from_utf8_lossy(&output.stdout);
        
        let mut current_device: Option<PeripheralDevice> = None;
        
        for line in info.lines() {
            let line = line.trim();
            if line.ends_with(":") && !line.contains("USB") {
                if let Some(device) = current_device.take() {
                    devices.push(device);
                }
                current_device = Some(PeripheralDevice {
                    id: format!("usb-{}", devices.len()),
                    name: line.trim_end_matches(':').to_string(),
                    device_type: "USB".to_string(),
                    manufacturer: String::new(),
                    serial_number: None,
                    connection_type: "USB".to_string(),
                    is_internal: false,
                    properties: Default::default(),
                    last_seen: Utc::now(),
                });
            } else if let Some(device) = &mut current_device {
                if line.starts_with("Manufacturer:") {
                    device.manufacturer = line.split(':').nth(1).unwrap_or("").trim().to_string();
                } else if line.starts_with("Serial Number:") {
                    device.serial_number = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
                } else if line.contains("Built-in") {
                    device.is_internal = true;
                }
                // Store additional properties
                if line.contains(":") {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        device.properties.insert(
                            parts[0].trim().to_string(),
                            parts[1].trim().to_string(),
                        );
                    }
                }
            }
        }

        if let Some(device) = current_device {
            devices.push(device);
        }

        // Get Bluetooth devices
        let output = Command::new("system_profiler")
            .args(["SPBluetoothDataType"])
            .output()?;
        let info = String::from_utf8_lossy(&output.stdout);
        
        for line in info.lines() {
            let line = line.trim();
            if line.contains("Connected:") && line.contains("Yes") {
                if let Some(name) = line.split(':').next() {
                    devices.push(PeripheralDevice {
                        id: format!("bt-{}", devices.len()),
                        name: name.trim().to_string(),
                        device_type: "Bluetooth".to_string(),
                        manufacturer: String::new(),
                        serial_number: None,
                        connection_type: "Bluetooth".to_string(),
                        is_internal: false,
                        properties: Default::default(),
                        last_seen: Utc::now(),
                    });
                }
            }
        }

        Ok(devices)
    }

    fn collect_power_info(&self) -> Result<PowerInfo> {
        let output = Command::new("pmset").arg("-g").arg("batt").output()?;
        let info = String::from_utf8_lossy(&output.stdout);
        
        let mut power_info = PowerInfo::default();
        
        for line in info.lines() {
            if line.contains("Now drawing from") {
                power_info.power_source = if line.contains("AC Power") {
                    "AC Power".to_string()
                } else if line.contains("Battery Power") {
                    "Battery".to_string()
                } else {
                    "Unknown".to_string()
                };
            } else if line.contains("%") {
                power_info.battery_present = true;
                if let Some(pct) = line.split('%').next() {
                    if let Ok(capacity) = pct.trim().parse::<u32>() {
                        power_info.battery_capacity = Some(capacity);
                    }
                }
                
                if line.contains("charging") {
                    power_info.charging = true;
                }
                
                // Parse time remaining
                if let Some(time_str) = line.split(';').nth(1) {
                    if let Some(mins) = time_str.trim().split_whitespace().next() {
                        if let Ok(minutes) = mins.parse::<u32>() {
                            power_info.time_remaining = Some(minutes);
                        }
                    }
                }
            }
        }

        // Get battery health information
        let health_output = Command::new("system_profiler")
            .args(["SPPowerDataType"])
            .output()?;
        let health_info = String::from_utf8_lossy(&health_output.stdout);
        
        for line in health_info.lines() {
            let line = line.trim();
            if line.starts_with("Cycle Count:") {
                if let Some(count_str) = line.split(':').nth(1) {
                    if let Ok(count) = count_str.trim().parse::<u32>() {
                        power_info.battery_cycle_count = Some(count);
                    }
                }
            } else if line.starts_with("Condition:") {
                power_info.battery_health = line.split(':').nth(1).map(|s| s.trim().to_string());
            }
        }

        Ok(power_info)
    }
} 