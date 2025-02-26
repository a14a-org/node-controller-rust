use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub collected_at: DateTime<Utc>,
    pub hostname: String,
    pub platform: PlatformInfo,
    pub hardware: HardwareInfo,
    pub peripherals: Vec<PeripheralDevice>,
    pub displays: Vec<DisplayInfo>,
    pub power: PowerInfo,
    #[serde(skip)]
    pub last_update: UpdateTracker,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateTracker {
    pub last_full_update: DateTime<Utc>,
    pub last_peripheral_check: DateTime<Utc>,
    pub last_power_check: DateTime<Utc>,
    pub changed_fields: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PlatformInfo {
    pub os_type: String,
    pub os_version: String,
    pub kernel_version: String,
    pub architecture: String,
    pub boot_time: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub available_memory: u64,
    pub total_memory: u64,
    pub load_average: (f64, f64, f64),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct HardwareInfo {
    pub model_name: String,
    pub model_identifier: String,
    pub processor_name: String,
    pub processor_speed: String,
    pub processor_count: u32,
    pub core_count: u32,
    pub memory_size: u64,
    pub memory_type: String,
    pub gpu_info: Vec<GpuInfo>,
    pub serial_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub memory_size: Option<u64>,
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PeripheralDevice {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub manufacturer: String,
    pub serial_number: Option<String>,
    pub connection_type: String,
    pub is_internal: bool,
    pub properties: HashMap<String, String>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DisplayInfo {
    pub name: String,
    pub resolution: (u32, u32),
    pub refresh_rate: f32,
    pub is_builtin: bool,
    pub serial_number: Option<String>,
    pub technology: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PowerInfo {
    pub power_source: String,
    pub battery_present: bool,
    pub battery_cycle_count: Option<u32>,
    pub battery_capacity: Option<u32>,
    pub battery_health: Option<String>,
    pub time_remaining: Option<u32>,
    pub charging: bool,
}

impl SystemInfo {
    pub fn new() -> Self {
        Self {
            collected_at: Utc::now(),
            hostname: String::new(),
            platform: PlatformInfo::default(),
            hardware: HardwareInfo::default(),
            peripherals: Vec::new(),
            displays: Vec::new(),
            power: PowerInfo::default(),
            last_update: UpdateTracker {
                last_full_update: Utc::now(),
                last_peripheral_check: Utc::now(),
                last_power_check: Utc::now(),
                changed_fields: Vec::new(),
            },
        }
    }

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
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self {
            os_type: String::new(),
            os_version: String::new(),
            kernel_version: String::new(),
            architecture: String::new(),
            boot_time: Utc::now(),
            uptime_seconds: 0,
            available_memory: 0,
            total_memory: 0,
            load_average: (0.0, 0.0, 0.0),
        }
    }
}

impl Default for HardwareInfo {
    fn default() -> Self {
        Self {
            model_name: String::new(),
            model_identifier: String::new(),
            processor_name: String::new(),
            processor_speed: String::new(),
            processor_count: 0,
            core_count: 0,
            memory_size: 0,
            memory_type: String::new(),
            gpu_info: Vec::new(),
            serial_number: None,
        }
    }
}

impl Default for PowerInfo {
    fn default() -> Self {
        Self {
            power_source: "Unknown".to_string(),
            battery_present: false,
            battery_cycle_count: None,
            battery_capacity: None,
            battery_health: None,
            time_remaining: None,
            charging: false,
        }
    }
}

impl std::fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "System Information:")?;
        writeln!(f, "  Hostname: {}", self.hostname)?;
        
        writeln!(f, "\nPlatform:")?;
        writeln!(f, "  OS: {} {}", self.platform.os_type, self.platform.os_version)?;
        writeln!(f, "  Kernel: {}", self.platform.kernel_version)?;
        writeln!(f, "  Architecture: {}", self.platform.architecture)?;
        writeln!(f, "  Boot Time: {}", self.platform.boot_time)?;
        writeln!(f, "  Uptime: {} hours", self.platform.uptime_seconds / 3600)?;
        writeln!(f, "  Memory: {} available of {}", 
            Self::format_size(self.platform.available_memory),
            Self::format_size(self.platform.total_memory))?;
        writeln!(f, "  Load Average: {:.2}, {:.2}, {:.2}", 
            self.platform.load_average.0,
            self.platform.load_average.1,
            self.platform.load_average.2)?;

        writeln!(f, "\nHardware:")?;
        writeln!(f, "  Model: {} ({})", self.hardware.model_name, self.hardware.model_identifier)?;
        writeln!(f, "  Processor: {} @ {}", self.hardware.processor_name, self.hardware.processor_speed)?;
        writeln!(f, "  Cores: {} physical, {} logical", 
            self.hardware.core_count,
            self.hardware.processor_count)?;
        writeln!(f, "  Memory: {} {}", 
            Self::format_size(self.hardware.memory_size),
            self.hardware.memory_type)?;
        
        if !self.hardware.gpu_info.is_empty() {
            writeln!(f, "\nGPUs:")?;
            for gpu in &self.hardware.gpu_info {
                writeln!(f, "  {} - {}", gpu.name, gpu.vendor)?;
                if let Some(mem) = gpu.memory_size {
                    writeln!(f, "    Memory: {}", Self::format_size(mem))?;
                }
            }
        }

        if !self.displays.is_empty() {
            writeln!(f, "\nDisplays:")?;
            for display in &self.displays {
                writeln!(f, "  {} - {}x{} @ {}Hz", 
                    display.name,
                    display.resolution.0,
                    display.resolution.1,
                    display.refresh_rate)?;
            }
        }

        writeln!(f, "\nPower:")?;
        writeln!(f, "  Source: {}", self.power.power_source)?;
        if self.power.battery_present {
            if let Some(capacity) = self.power.battery_capacity {
                writeln!(f, "  Battery: {}%", capacity)?;
            }
            if let Some(health) = &self.power.battery_health {
                writeln!(f, "  Health: {}", health)?;
            }
            if let Some(time) = self.power.time_remaining {
                writeln!(f, "  Time Remaining: {} minutes", time)?;
            }
        }

        if !self.peripherals.is_empty() {
            writeln!(f, "\nPeripherals:")?;
            for device in &self.peripherals {
                write!(f, "  {} ({}) - {}", device.name, device.device_type, device.manufacturer)?;
                if device.is_internal {
                    write!(f, " [Internal]")?;
                }
                writeln!(f)?;
            }
        }

        Ok(())
    }
} 